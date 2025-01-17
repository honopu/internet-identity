import {
  handleLogin,
  handleLoginFlowResult,
} from "$src/components/authenticateBox";
import { addDeviceSuccess } from "$src/flows/addDevice/manage/addDeviceSuccess";
import { nonNullish } from "@dfinity/utils";
import { registerTentativeDevice } from "./flows/addDevice/welcomeView/registerTentativeDevice";
import { authFlowAuthorize } from "./flows/authorize";
import { authFlowManage, renderManageWarmup } from "./flows/manage";
import { createSpa } from "./spa";
import { getAddDeviceAnchor } from "./utils/addDeviceLink";

void createSpa(async (connection) => {
  // Figure out if user is trying to add a device. If so, use the anchor from the URL.
  const addDeviceAnchor = getAddDeviceAnchor();
  if (nonNullish(addDeviceAnchor)) {
    const userNumber = addDeviceAnchor;
    // Register this device (tentatively)
    const { alias: deviceAlias } = await registerTentativeDevice(
      addDeviceAnchor,
      connection
    );

    // Display a success page once device added (above registerTentativeDevice **never** returns if it fails)
    await addDeviceSuccess({ deviceAlias });

    const renderManage = renderManageWarmup();

    // If user "Click" continue in success page, proceed with authentication
    const result = await handleLogin({
      login: () => connection.login(userNumber),
    });
    const loginData = await handleLoginFlowResult(result);

    // User have successfully signed-in we can jump to manage page
    if (nonNullish(loginData)) {
      return await renderManage({
        userNumber,
        connection: loginData.connection,
      });
    }
  }

  const url = new URL(document.URL);

  // Simple, #-based routing
  if (url.hash === "#authorize") {
    // User was brought here by a dapp for authorization
    return authFlowAuthorize(connection);
  } else {
    // The default flow
    return authFlowManage(connection);
  }
});
