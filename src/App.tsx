import { AppProviders } from "@/app/providers/AppProviders";
import { AppRoutes } from "@/app/routes";
import { AuthProvider } from "@/stores/authContext";
import { AuthGate } from "@/app/AuthGate";

export default function App() {
  return (
    <AppProviders>
      <AuthProvider>
        <AuthGate>
          <AppRoutes />
        </AuthGate>
      </AuthProvider>
    </AppProviders>
  );
}
