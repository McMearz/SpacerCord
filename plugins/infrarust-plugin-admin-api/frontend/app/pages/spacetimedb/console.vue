<script setup lang="ts">
import {
  PauseIcon,
  PlayIcon,
  TrashIcon,
  ClipboardDocumentIcon,
  CircleStackIcon,
  TableCellsIcon,
  CodeBracketIcon,
  CommandLineIcon,
  DocumentTextIcon,
  SignalIcon,
} from '@heroicons/vue/24/outline';
import type { LogEntryDto } from '~/types/api';
import { useStdbLogStream } from '~/composables/useStdbLogStream';

const {
  logs,
  status: sseStatus,
  paused,
  pause: pauseStream,
  resume: resumeStream,
  clear: clearLogs,
} = useStdbLogStream();

const { push } = useToast();

const subNav = [
  { to: '/spacetimedb', label: 'Overview', icon: CircleStackIcon },
  { to: '/spacetimedb/tables', label: 'Tables', icon: TableCellsIcon },
  { to: '/spacetimedb/reducers', label: 'Reducers', icon: CodeBracketIcon },
  { to: '/spacetimedb/sql', label: 'SQL Console', icon: CommandLineIcon },
  { to: '/spacetimedb/events', label: 'Event Feed', icon: SignalIcon },
  { to: '/spacetimedb/console', label: 'Live Logs', icon: DocumentTextIcon },
];

function togglePause() {
  if (paused.value) resumeStream();
  else pauseStream();
}

function copyLogs() {
  const text = logs.value.map((l: LogEntryDto) => `${l.timestamp} [${l.target}] ${l.message}`).join('\n');
  navigator.clipboard.writeText(text);
  push({ type: 'success', title: 'Copied to clipboard' });
}
</script>

<template>
  <div class="flex h-[calc(100vh-var(--ir-ticker-height)-3rem)] flex-col gap-3 lg:h-[calc(100vh-var(--ir-ticker-height)-3.5rem)]">
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

    <!-- Top bar -->
    <div class="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
      <div class="flex items-center gap-3">
        <h2 class="text-xl font-bold tracking-tight">Managed Logs</h2>
        <div class="flex items-center gap-1.5 rounded-full border border-[var(--ir-border)] bg-[var(--ir-surface-soft)] px-2.5 py-1">
          <span
            class="h-1.5 w-1.5 rounded-full"
            :class="sseStatus === 'open' && !paused ? 'bg-[var(--ir-success)] status-pulse' : paused ? 'bg-[var(--ir-warn)]' : 'bg-slate-500'"
          />
          <span class="text-[10px] font-medium uppercase tracking-wider text-[var(--ir-text-muted)]">
            {{ paused ? 'Paused' : sseStatus === 'open' ? 'Streaming' : 'Connecting' }}
          </span>
        </div>
      </div>

      <div class="flex items-center gap-1.5">
        <button class="btn btn-ghost flex items-center gap-1.5 text-xs" @click="togglePause">
          <component :is="paused ? PlayIcon : PauseIcon" class="h-4 w-4" />
          {{ paused ? 'Resume' : 'Pause' }}
        </button>
        <button class="btn btn-ghost flex items-center gap-1.5 text-xs" @click="clearLogs">
          <TrashIcon class="h-4 w-4" />
          Clear
        </button>
        <button class="btn btn-ghost flex items-center gap-1.5 text-xs" @click="copyLogs">
          <ClipboardDocumentIcon class="h-4 w-4" />
          Copy
        </button>
      </div>
    </div>

    <!-- Log console -->
    <div class="min-h-0 flex-1">
      <LogConsole :logs="logs" full-height />
    </div>
  </div>
</template>
