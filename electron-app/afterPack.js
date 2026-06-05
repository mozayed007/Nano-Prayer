const { flipFuses, FuseVersion, FuseV1Options } = require('@electron/fuses');
const path = require('path');
const fs = require('fs');

exports.default = async function(context) {
  const { appOutDir, packager } = context;
  const exeName = packager.appInfo.productName || 'NanoPrayReminder-Electron';
  const exePath = path.join(appOutDir, `${exeName}.exe`);
  if (!fs.existsSync(exePath)) {
    console.warn(`[afterPack] Electron binary not found at ${exePath}, skipping fuses.`);
    return;
  }
  await flipFuses(exePath, {
    version: FuseVersion.V1,
    [FuseV1Options.RunAsNode]: false,
    [FuseV1Options.EnableCookieEncryption]: true,
    [FuseV1Options.EnableNodeOptionsEnvironmentVariable]: false,
    [FuseV1Options.EnableNodeCliInspectArguments]: false,
  });
  console.log(`[afterPack] Fuses applied to ${exePath}`);
};
