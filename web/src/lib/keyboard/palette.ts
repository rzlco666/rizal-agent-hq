export interface PaletteAction {
  id: string;
  label: string;
  group: 'navigate' | 'agent' | 'alert' | 'approval' | 'view' | 'setting';
  keywords?: string[];
  shortcut?: string;
  disabledReason?: string;
  run: () => void | Promise<void>;
}
