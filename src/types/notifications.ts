export interface Notification {
  id: number;
  title: string;
  message: string;
  kind: string;
  route: string;
  routeQuery: string | null;
  createdAt: string;
  read: boolean;
}
