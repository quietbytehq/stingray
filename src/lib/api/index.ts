import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AppError,
  AppInfo,
  ConnectionTest,
  FileEntry,
  ItemRef,
  LocalEntry,
  NewRemote,
  NfsShare,
  NfsShareView,
  NfsSupport,
  Place,
  ProviderDef,
  RcdState,
  RemoteForm,
  RemoteSummary,
  Settings,
  SplashPlan,
  TransferItem,
  TransferRequest,
} from "./types";

export type * from "./types";

export const RCD_STATE_EVENT = "rcd://state";
export const SIGN_IN_URL_EVENT = "wizard://sign-in-url";
export const TRANSFERS_EVENT = "transfers://update";

/** Typed wrappers around Tauri commands. Each one rejects with an AppError on failure. */
export const api = {
  getRcdState: () => invoke<RcdState>("get_rcd_state"),
  restartRcd: () => invoke<void>("restart_rcd"),
  splashFinished: () => invoke<void>("splash_finished"),
  getSplashPlan: () => invoke<SplashPlan>("get_splash_plan"),
  appInfo: () => invoke<AppInfo>("app_info"),
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) => invoke<Settings>("save_settings", { settings }),

  listProviders: () => invoke<ProviderDef[]>("list_providers"),
  listRemotes: () => invoke<RemoteSummary[]>("list_remotes"),
  getRemoteForm: (name: string) => invoke<RemoteForm>("get_remote_form", { name }),
  createRemote: (request: NewRemote) => invoke<RemoteSummary>("create_remote", { request }),
  startBrowserSignIn: (request: NewRemote) => invoke<RemoteSummary>("start_browser_sign_in", { request }),
  cancelBrowserSignIn: () => invoke<void>("cancel_browser_sign_in"),
  updateRemote: (name: string, values: Record<string, string>) =>
    invoke<void>("update_remote", { name, update: { values } }),
  renameRemote: (name: string, newName: string) => invoke<string>("rename_remote", { name, newName }),
  deleteRemote: (name: string) => invoke<void>("delete_remote", { name }),
  testRemote: (name: string) => invoke<ConnectionTest>("test_remote", { name }),

  inspectLocalPaths: (paths: string[]) => invoke<LocalEntry[]>("inspect_local_paths", { paths }),
  listFiles: (remote: string, path: string) => invoke<FileEntry[]>("list_files", { remote, path }),
  createFolder: (remote: string, path: string) => invoke<void>("create_folder", { remote, path }),
  deleteItems: (remote: string, items: ItemRef[]) => invoke<void>("delete_items", { remote, items }),
  renameItem: (remote: string, path: string, isDir: boolean, newName: string) =>
    invoke<string>("rename_item", { remote, path, isDir, newName }),
  enqueueTransfers: (requests: TransferRequest[]) => invoke<number[]>("enqueue_transfers", { requests }),
  listTransfers: () => invoke<TransferItem[]>("list_transfers"),
  cancelTransfer: (id: number) => invoke<void>("cancel_transfer", { id }),
  clearFinishedTransfers: () => invoke<void>("clear_finished_transfers"),

  homePlaces: () => invoke<Place[]>("home_places"),
  listLocal: (path: string) => invoke<FileEntry[]>("list_local", { path }),
  localCreateFolder: (parent: string, name: string) => invoke<string>("local_create_folder", { parent, name }),
  localRename: (path: string, newName: string) => invoke<string>("local_rename", { path, newName }),
  localTrash: (paths: string[]) => invoke<void>("local_trash", { paths }),
  openLocal: (path: string) => invoke<void>("open_local", { path }),

  nfsSupport: () => invoke<NfsSupport>("nfs_support"),
  listNfsShares: () => invoke<NfsShareView[]>("list_nfs_shares"),
  addNfsShare: (share: NfsShare) => invoke<NfsShare>("add_nfs_share", { share }),
  connectNfsShare: (name: string) => invoke<string>("connect_nfs_share", { name }),
  disconnectNfsShare: (name: string) => invoke<void>("disconnect_nfs_share", { name }),
  removeNfsShare: (name: string) => invoke<void>("remove_nfs_share", { name }),
};

export function onRcdState(handler: (state: RcdState) => void): Promise<UnlistenFn> {
  return listen<RcdState>(RCD_STATE_EVENT, (event) => handler(event.payload));
}

export function onTransfers(handler: (items: TransferItem[]) => void): Promise<UnlistenFn> {
  return listen<TransferItem[]>(TRANSFERS_EVENT, (event) => handler(event.payload));
}


export function onSignInUrl(handler: (url: string) => void): Promise<UnlistenFn> {
  return listen<string>(SIGN_IN_URL_EVENT, (event) => handler(event.payload));
}

export function isAppError(value: unknown): value is AppError {
  return (
    typeof value === "object" &&
    value !== null &&
    "kind" in value &&
    "message" in value &&
    typeof (value as { message: unknown }).message === "string"
  );
}

/** Turns anything thrown by a command into text fit for the UI. */
export function errorMessage(err: unknown): string {
  if (isAppError(err)) return err.message;
  if (err instanceof Error) return err.message;
  return String(err);
}

/**
 * Turns anything thrown into an AppError, so the UI can show the plain message and keep the
 * technical text behind "Show details". Unknown errors are "internal" with the raw text as
 * details and a generic message.
 */
export function asAppError(err: unknown): AppError {
  if (isAppError(err)) return err;
  const raw = err instanceof Error ? err.message : String(err);
  return { kind: "internal", message: "Something went wrong inside Stingray. Try again.", details: raw };
}
