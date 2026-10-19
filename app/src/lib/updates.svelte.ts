import { api, errorText, type UpdateInfo } from "./api";

/** Update state, shared by the sidebar and the settings page. */
export const updates = $state({
  available: null as UpdateInfo | null,
  checking: false,
  installing: false,
  /** A check finished in this session (to say "up to date"). */
  checked: false,
  error: null as string | null,
});

export async function checkForUpdate() {
  if (updates.checking || updates.installing) return;
  updates.checking = true;
  updates.error = null;
  try {
    updates.available = await api.checkUpdate();
    updates.checked = true;
  } catch (e) {
    updates.error = errorText(e);
  } finally {
    updates.checking = false;
  }
}

/** Downloads and runs the installer: PeriHub closes and the new version starts. */
export async function installUpdate() {
  updates.installing = true;
  updates.error = null;
  try {
    await api.installUpdate();
  } catch (e) {
    updates.error = errorText(e);
    updates.installing = false;
  }
}
