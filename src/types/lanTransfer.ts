export interface LanDevice { device_id:string; device_name:string; ip_address:string; app_version:string; protocol:string; }
export interface PairingRequest { device_id:string; device_name:string; ip_address:string; app_version:string; requested_at:string; }
export interface PairingSession { code:string; nonce:string; expires_seconds:number; device_id:string; device_name:string; }

export interface LanTransferHistory { id:number; direction:string; file_name:string; file_size:number; status:string; created_at:string; device_name:string; }
