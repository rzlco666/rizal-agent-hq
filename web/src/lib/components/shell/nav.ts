import {
  Bell,
  BookOpen,
  Bot,
  Building2,
  CalendarDays,
  CircleDollarSign,
  History,
  Plug,
  Settings2,
  Workflow
} from 'lucide-svelte';
import type { ComponentType } from 'svelte';
import type { TranslationKey } from '$lib/i18n';

export interface NavItem {
  id: string;
  href: string;
  labelKey: TranslationKey;
  icon: ComponentType;
}

export const navItems: NavItem[] = [
  { id: 'today', href: '/', labelKey: 'nav.today', icon: CalendarDays },
  { id: 'agents', href: '/agents', labelKey: 'nav.agents', icon: Bot },
  { id: 'office', href: '/office', labelKey: 'nav.office', icon: Building2 },
  { id: 'timeline', href: '/timeline', labelKey: 'nav.timeline', icon: History },
  { id: 'alerts', href: '/alerts', labelKey: 'nav.alerts', icon: Bell },
  { id: 'automations', href: '/automations', labelKey: 'nav.automations', icon: Workflow },
  { id: 'integrations', href: '/integrations', labelKey: 'nav.integrations', icon: Plug },
  { id: 'knowledge', href: '/knowledge', labelKey: 'nav.knowledge', icon: BookOpen },
  { id: 'costs', href: '/costs', labelKey: 'nav.costs', icon: CircleDollarSign },
  { id: 'settings', href: '/settings', labelKey: 'nav.settings', icon: Settings2 }
];

export const mobileNavIds = ['today', 'alerts', 'office', 'costs'];

export function isActive(pathname: string, href: string): boolean {
  return href === '/' ? pathname === '/' : pathname === href || pathname.startsWith(`${href}/`);
}
