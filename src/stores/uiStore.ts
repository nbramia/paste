export type ActiveView = "history" | "pinboards" | "snippets" | "settings";

export interface UiState {
  activeView: ActiveView;
  selectedIndex: number;
  isOverlayVisible: boolean;
}

export const initialUiState: UiState = {
  activeView: "history",
  selectedIndex: 0,
  isOverlayVisible: false,
};
