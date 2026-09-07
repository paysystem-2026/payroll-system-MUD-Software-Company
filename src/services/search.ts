import { invokeCommand } from "@/services/tauri";

export interface GlobalSearchResult {
  table: string;
  record_id: number | null;
  title: string;
  subtitle: string;
  matched_field: string;
  matched_value: string;
  route: string;
}

export const searchService = {
  search: (token: string, query: string, limit = 50) =>
    invokeCommand<GlobalSearchResult[]>("global_search", { token, query, limit }, []),
};
