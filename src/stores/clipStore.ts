export interface ClipState {
  clips: never[];
  loading: boolean;
}

export const initialClipState: ClipState = {
  clips: [],
  loading: false,
};
