export type ToastTone = 'success' | 'critical' | 'info';

export interface ToastOptions {
  tone: ToastTone;
  message: string;
  action?: { label: string; run: () => void };
  timeoutMs?: number;
}

export interface ToastItem extends ToastOptions {
  id: string;
}

let counter = 0;

class ToastStore {
  items = $state<ToastItem[]>([]);

  show(options: ToastOptions): string {
    const id = `toast-${++counter}`;
    const next = [...this.items, { ...options, id }];
    while (next.length > 3) {
      const oldestSuccess = next.findIndex((item) => item.tone === 'success');
      next.splice(oldestSuccess >= 0 ? oldestSuccess : 0, 1);
    }
    this.items = next;
    return id;
  }

  dismiss(id: string) {
    this.items = this.items.filter((item) => item.id !== id);
  }
}

export const toasts = new ToastStore();

export function showToast(options: ToastOptions): string {
  return toasts.show(options);
}

export function dismissToast(id: string): void {
  toasts.dismiss(id);
}
