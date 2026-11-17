import { ackAlert, resolveAlert, snoozeAlert, type AlertView } from '$lib/api/client';
import { formatRelative } from '$lib/format';
import { t } from '$lib/i18n';
import { showToast } from './toast.svelte';

export interface AttentionItem {
  id: string;
  kind: 'approval' | 'alert';
  level?: 'warning' | 'critical';
  title: string;
  meta: string;
  href?: string;
}

function fromAlert(alert: AlertView): AttentionItem | null {
  const level = alert.level === 'critical' || alert.level === 'warning' ? alert.level : null;
  if (!level) return null;
  return {
    id: alert.id,
    kind: 'alert',
    level,
    title: alert.title || alert.rule_id,
    meta: `${alert.subject} · ${formatRelative(new Date(alert.opened_ms))}`,
    href: `/alerts/${encodeURIComponent(alert.id)}`
  };
}

class AttentionStore {
  items = $state<AttentionItem[]>([]);
  openCount = $state(0);

  setAlerts(alerts: AlertView[], openCount: number) {
    this.items = alerts.map(fromAlert).filter((item): item is AttentionItem => item !== null);
    this.openCount = openCount;
  }

  async ack(id: string) {
    const index = this.items.findIndex((item) => item.id === id);
    if (index < 0) return;
    const previous = this.items[index];
    const previousCount = this.openCount;
    this.items = this.items.filter((item) => item.id !== id);
    this.openCount = Math.max(0, this.openCount - 1);
    try {
      await ackAlert(id);
    } catch {
      const restored = [...this.items];
      restored.splice(index, 0, previous);
      this.items = restored;
      this.openCount = previousCount;
      showToast({ tone: 'critical', message: t('toast.ack_failed') });
    }
  }

  async snooze(id: string) {
    const index = this.items.findIndex((item) => item.id === id);
    if (index < 0) return;
    const previous = this.items[index];
    const previousCount = this.openCount;
    this.items = this.items.filter((item) => item.id !== id);
    this.openCount = Math.max(0, this.openCount - 1);
    try {
      await snoozeAlert(id, 60);
    } catch {
      const restored = [...this.items];
      restored.splice(index, 0, previous);
      this.items = restored;
      this.openCount = previousCount;
      showToast({ tone: 'critical', message: t('toast.ack_failed') });
    }
  }

  async resolve(id: string) {
    await resolveAlert(id);
    this.items = this.items.filter((item) => item.id !== id);
    this.openCount = Math.max(0, this.openCount - 1);
  }
}

export const attention = new AttentionStore();
