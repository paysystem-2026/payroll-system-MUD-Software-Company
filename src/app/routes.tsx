import { Routes, Route } from "react-router-dom";
import { AppLayout } from "@/app/layouts/AppLayout";
import { DashboardPage } from "@/modules/dashboard/DashboardPage";
import { BasicDataPage } from "@/modules/basic-data/BasicDataPage";
import { StaffPage } from "@/modules/staff/StaffPage";
import { LeavesPage } from "@/modules/leaves/LeavesPage";
import { PayrollPage } from "@/modules/payroll/PayrollPage";
import { ReportsPage } from "@/modules/reports/ReportsPage";
import { RemindersPage } from "@/modules/reminders/RemindersPage";
import { BackupPage } from "@/modules/backup/BackupPage";
import { LanTransferPage } from "@/modules/lan-transfer/LanTransferPage";
import { UpdatesPage } from "@/modules/updates/UpdatesPage";
import { AdministrationPage } from "@/modules/administration/AdministrationPage";
import { SettingsPage } from "@/modules/settings/SettingsPage";

export function AppRoutes() {
  return (
    <Routes>
      <Route element={<AppLayout />}>
        <Route path="/" element={<DashboardPage />} />
        <Route path="/basic-data" element={<BasicDataPage />} />
        <Route path="/staff" element={<StaffPage />} />
        <Route path="/leaves" element={<LeavesPage />} />
        <Route path="/payroll" element={<PayrollPage />} />
        <Route path="/reports" element={<ReportsPage />} />
        <Route path="/reminders" element={<RemindersPage />} />
        <Route path="/backup" element={<BackupPage />} />
        <Route path="/lan-transfer" element={<LanTransferPage />} />
        <Route path="/updates" element={<UpdatesPage />} />
        <Route path="/administration" element={<AdministrationPage />} />
        <Route path="/settings" element={<SettingsPage />} />
      </Route>
    </Routes>
  );
}
