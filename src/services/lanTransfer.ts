import { invokeCommand } from "@/services/tauri";
import type { LanDevice, PairingRequest, PairingSession } from "@/types/lanTransfer";

const requireDesktop = () => {
  if (typeof window === "undefined" || !(window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__) {
    throw new Error("LAN Transfer requires the Tauri desktop application and the real local-network backend.");
  }
};

export const lanTransferService = {
  getDevice() { requireDesktop(); return invokeCommand<LanDevice>("get_lan_device", {}, null as unknown as LanDevice); },
  discover() { requireDesktop(); return invokeCommand<LanDevice[]>("discover_lan_devices", {}, []); },
  requestPairing(device: LanDevice) { requireDesktop(); return invokeCommand<PairingSession>("request_lan_pairing", { device }, null as unknown as PairingSession); },
  pairingRequests() { requireDesktop(); return invokeCommand<PairingRequest[]>("get_lan_pairing_requests", {}, []); },
  approvePairing(deviceId: string, code: string) { requireDesktop(); return invokeCommand<{state:string;device_id:string|null;device_name:string|null;message:string}>("approve_lan_pairing", { device_id: deviceId, code }, null as unknown as {state:string;device_id:string|null;device_name:string|null;message:string}); },
  pairedDevices() { requireDesktop(); return invokeCommand<LanDevice[]>("get_paired_lan_devices", {}, []); },
  sendBackup(backupId: number, deviceId: string) { requireDesktop(); return invokeCommand<void>("send_lan_backup", { backup_id: backupId, device_id: deviceId }, undefined); },
  testConnection(deviceId: string) { requireDesktop(); return invokeCommand<LanDevice>("test_lan_connection", { device_id: deviceId }, null as unknown as LanDevice); },
  revokeDevice(deviceId: string) { requireDesktop(); return invokeCommand<void>("revoke_lan_device", { device_id: deviceId }, undefined); },
  importBackup(token: string, backupId: number) { requireDesktop(); return invokeCommand<{success:boolean;message:string}>("import_lan_backup", { token, backup_id: backupId }, null as unknown as {success:boolean;message:string}); },
  history() { requireDesktop(); return invokeCommand<Array<{id:number;direction:string;file_name:string;file_size:number;status:string;created_at:string;device_name:string}>>("get_lan_transfer_history", {}, []); },
};
