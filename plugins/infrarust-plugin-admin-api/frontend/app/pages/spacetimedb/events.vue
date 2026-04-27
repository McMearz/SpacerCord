<script setup lang="ts">
import {
  CircleStackIcon,
  TableCellsIcon,
  CodeBracketIcon,
  CommandLineIcon,
  DocumentTextIcon,
  SignalIcon,
  TrashIcon,
  PauseIcon,
  PlayIcon,
} from '@heroicons/vue/24/outline';
import { useStdbEventStream } from '~/composables/useStdbEventStream';

const { events, status: sseStatus, paused, togglePause, clear } = useStdbEventStream();

const subNav = [
  { to: '/spacetimedb', label: 'Overview', icon: CircleStackIcon },
  { to: '/spacetimedb/tables', label: 'Tables', icon: TableCellsIcon },
  { to: '/spacetimedb/reducers', label: 'Reducers', icon: CodeBracketIcon },
  { to: '/spacetimedb/sql', label: 'SQL Console', icon: CommandLineIcon },
  { to: '/spacetimedb/events', label: 'Event Feed', icon: SignalIcon },
  { to: '/spacetimedb/console', label: 'Live Logs', icon: DocumentTextIcon },
];

function opClass(op: string) {
  switch (op.toLowerCase()) {
    case 'insert': return 'text-green-400 bg-green-400/10';
    case 'update': return 'text-blue-400 bg-blue-400/10';
    case 'delete': return 'text-red-400 bg-red-400/10';
    default: return 'text-slate-400 bg-slate-400/10';
  }
}
</script>

<template>
  <div class="grid gap-5">
    <!-- Sub Nav -->
    <div class="flex flex-wrap gap-2">
      <NuxtLink
        v-for="item in subNav"
        :key="item.to"
        :to="item.to"
        class="flex items-center gap-2 rounded-md border border-[var(--ir-border)] px-3 py-1.5 text-sm font-medium transition-all hover:bg-white/[0.04]"
        :class="$route.path === item.to ? 'bg-[var(--ir-accent-soft)] border-[var(--ir-accent-soft)] text-white' : 'text-[var(--ir-text-muted)]'"
      >
        <component :is="item.icon" class="h-4 w-4" />
        {{ item.label }}
      </NuxtLink>
    </div>

    <div class="glass-pane flex flex-col min-h-[600px]">
      <header class="p-4 border-b border-white/[0.03] flex items-center justify-between">
        <div class="flex items-center gap-3">
          <h2 class="text-xl font-bold tracking-tight">Row Change Events</h2>
          <div class="flex items-center gap-1.5 rounded-full border border-[var(--ir-border)] bg-[var(--ir-surface-soft)] px-2.5 py-1">
            <span
              class="h-1.5 w-1.5 rounded-full"
              :class="sseStatus === 'open' && !paused ? 'bg-[var(--ir-success)] status-pulse' : paused ? 'bg-[var(--ir-warn)]' : 'bg-slate-500'"
            />
            <span class="text-[10px] font-medium uppercase tracking-wider text-[var(--ir-text-muted)]">
              {{ paused ? 'Paused' : sseStatus === 'open' ? 'Live' : 'Connecting' }}
            </span>
          </div>
        </div>
        <div class="flex gap-2">
          <button class="btn btn-ghost btn-xs" @click="togglePause">
            <component :is="paused ? PlayIcon : PauseIcon" class="h-4 w-4" />
          </button>
          <button class="btn btn-ghost btn-xs" @click="clear">
            <TrashIcon class="h-4 w-4" />
          </button>
        </div>
      </header>

      <div class="flex-1 overflow-y-auto">
        <div v-if="events.length === 0" class="flex flex-col items-center justify-center py-20 text-[var(--ir-text-muted)] opacity-50">
          <SignalIcon class="h-12 w-12 mb-4" />
          <p>Waiting for database events...</p>
        </div>
        
        <div class="divide-y divide-white/[0.03]">
          <div v-for="(ev, i) in events" :key="i" class="p-4 hover:bg-white/[0.01] transition-colors flex items-center gap-4">
            <div class="shrink-0 w-32 text-[10px] font-mono text-[var(--ir-text-muted)]">
              {{ new Date(ev.timestamp).toLocaleTimeString() }}
            </div>
            <div class="shrink-0">
               <span class="px-2 py-0.5 rounded text-[10px] font-bold uppercase tracking-wider" :class="opClass(ev.operation)">
                 {{ ev.operation }}
               </span>
            </div>
            <div class="flex-1 font-medium text-sm">
              {{ ev.table_name }}
            </div>
            <div class="text-xs font-mono text-[var(--ir-text-muted)] bg-white/[0.03] px-2 py-1 rounded">
              {{ ev.data_len }} bytes
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
