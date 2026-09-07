import { invokeCommand } from "@/services/tauri";

export interface GeneralSettings {
  app_name: string;
  language: string;
  currency: string;
  date_format: string;
  number_format: string;
  payroll_period_behavior: string;
}

export interface UpdateSettings {
  auto_check: boolean;
  channel: string;
}

export interface ReminderSettings {
  notifications_enabled: boolean;
  polling_interval_seconds: number;
  default_snooze_minutes: number;
  default_category: string;
  show_completed: boolean;
  auto_advance_recurring: boolean;
}

interface SettingsResponse {
  success: boolean;
  message: string;
}

const defaultGeneral: GeneralSettings = {
  app_name: "Payroll System",
  language: "en",
  currency: "RWF",
  date_format: "DD/MM/YYYY",
  number_format: "1,234.56",
  payroll_period_behavior: "monthly",
};

const defaultUpdate: UpdateSettings = { auto_check: true, channel: "stable" };
const defaultReminder: ReminderSettings = {
  notifications_enabled: true, polling_interval_seconds: 15, default_snooze_minutes: 60,
  default_category: "general", show_completed: true, auto_advance_recurring: true,
};

export const settingsService = {
  getGeneralSettings() {
    return invokeCommand<GeneralSettings>("get_general_settings", {}, defaultGeneral);
  },
  updateGeneralSettings(token: string, settings: GeneralSettings) {
    return invokeCommand<SettingsResponse>("update_general_settings", { token, settings }, { success: true, message: "Saved in preview" });
  },
  getUpdateSettings() {
    return invokeCommand<UpdateSettings>("get_update_settings", {}, defaultUpdate);
  },
  updateUpdateSettings(token: string, settings: UpdateSettings) {
    return invokeCommand<SettingsResponse>("update_update_settings", { token, settings }, { success: true, message: "Saved in preview" });
  },
  getReminderSettings() {
    return invokeCommand<ReminderSettings>("get_reminder_settings", {}, defaultReminder);
  },
  updateReminderSettings(token: string, settings: ReminderSettings) {
    return invokeCommand<SettingsResponse>("update_reminder_settings", { token, settings }, { success: true, message: "Saved in preview" });
  },
  resetSystem(token: string) {
    return invokeCommand<SettingsResponse>("reset_system", { token }, { success: false, message: "System reset is available in the desktop app." });
  },
};
