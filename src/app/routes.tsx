import { lazy } from "react";
import { Routes, Route } from "react-router-dom";
import { AppLayout } from "@/app/layouts/AppLayout";

const DashboardPage = lazy(() => import("@/modules/dashboard/DashboardPage").then(m => ({ default: m.DashboardPage })));
const BasicDataPage = lazy(() => import("@/modules/basic-data/BasicDataPage").then(m => ({ default: m.BasicDataPage })));
const StaffPage = lazy(() => import("@/modules/staff/StaffPage").then(m => ({ default: m.StaffPage })));
const LeavesPage = lazy(() => import("@/modules/leaves/LeavesPage").then(m => ({ default: m.LeavesPage })));
const PayrollPage = lazy(() => import("@/modules/payroll/PayrollPage").then(m => ({ default: m.PayrollPage })));
const ReportsPage = lazy(() => import("@/modules/reports/ReportsPage").then(m => ({ default: m.ReportsPage })));
const RemindersPage = lazy(() => import("@/modules/reminders/RemindersPage").then(m => ({ default: m.RemindersPage })));
const BackupPage = lazy(() => import("@/modules/backup/BackupPage").then(m => ({ default: m.BackupPage })));
const LanTransferPage = lazy(() => import("@/modules/lan-transfer/LanTransferPage").then(m => ({ default: m.LanTransferPage })));
const UpdatesPage = lazy(() => import("@/modules/updates/UpdatesPage").then(m => ({ default: m.UpdatesPage })));
const AdministrationPage = lazy(() => import("@/modules/administration/AdministrationPage").then(m => ({ default: m.AdministrationPage })));
const SettingsPage = lazy(() => import("@/modules/settings/SettingsPage").then(m => ({ default: m.SettingsPage })));

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
