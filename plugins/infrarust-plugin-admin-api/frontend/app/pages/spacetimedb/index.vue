<script setup lang="ts">
import {
  ArrowPathIcon,
  CircleStackIcon,
  TableCellsIcon,
  CodeBracketIcon,
  CommandLineIcon,
  DocumentTextIcon,
  SignalIcon,
} from '@heroicons/vue/24/outline';
import { useSpacetimedb } from '~/composables/useSpacetimedb';

const { status, loading, error, restart, publish } = useSpacetimedb();
const { push } = useToast();
const { ask } = useConfirm();

async function handleRestart() {
  const confirmed = await ask('Restart SpacetimeDB', 'This will stop the current child process and re-run the boot sequence. Continue?');
  if (confirmed) {
    try {
      await restart();
      push({ type: 'success', title: 'Restart initiated' });
    } catch (e: any) {
      push({ type: 'error', title: e.message || 'Restart failed' });
    }
  }
}

async function handlePublish() {
  const confirmed = await ask('Publish Module', 'This will re-publish the bundled module to SpacetimeDB. Continue?');
  if (confirmed) {
    try {
      await publish();
      push({ type: 'success', title: 'Publish successful' });
    } catch (e: any) {
      push({ type: 'error', title: e.message || 'Publish failed' });
    }
  }
}

const subNav = [
  { to: '/spacetimedb', label: 'Overview', icon: CircleStackIcon },
  { to: '/spacetimedb/tables', label: 'Tables', icon: TableCellsIcon },
  { to: '/spacetimedb/reducers', label: 'Reducers', icon: CodeBracketIcon },
  { to: '/spacetimedb/sql', label: 'SQL Console', icon: CommandLineIcon },
  { to: '/spacetimedb/events', label: 'Event Feed', icon: SignalIcon },
  { to: '/spacetimedb/console', label: 'Live Logs', icon: DocumentTextIcon },
];
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

    <div v-if="!status?.enabled" class="glass-pane flex flex-col items-center justify-center p-12 text-center">
      <div class="mb-4 rounded-full bg-white/[0.03] p-4">
        <CircleStackIcon class="h-12 w-12 text-[var(--ir-text-muted)] opacity-20" />
      </div>
      <h2 class="text-xl font-bold">SpacetimeDB is Disabled</h2>
      <p class="mt-2 max-w-md text-[var(--ir-text-muted)]">
        The SpacetimeDB integration is currently disabled in your <code class="text-[var(--ir-accent)]">infrarust.toml</code>.
        Enable it to unlock persistent player data and real-time database management.
      </p>
    </div>

    <template v-else>
      <!-- Metric tiles -->
      <section class="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
        <StatCard label="Server State" :value="status?.server_state ?? '—'" :hint="status?.pid ? `PID: ${status.pid}` : 'Process status'" />
        <StatCard label="Connection" :value="status?.connection_state ?? '—'" hint="SDK connection status" />
        <StatCard label="Database" :value="status?.db_name ?? '—'" hint="Module name" />
        <StatCard label="Published" :value="status?.module_published ? 'Yes' : 'No'" hint="Module status" />
      </section>

      <section class="grid gap-4 lg:grid-cols-2">
        <!-- Connection Details -->
        <div class="glass-pane p-5">
          <h2 class="mb-4 text-sm font-semibold uppercase tracking-[0.08em]">Runtime Info</h2>
          <dl class="space-y-3">
            <div class="flex justify-between border-b border-white/[0.03] pb-2">
              <dt class="text-sm text-[var(--ir-text-muted)]">Listen Address</dt>
              <dd class="font-mono text-sm">{{ status?.listen }}</dd>
            </div>
            <div class="flex justify-between border-b border-white/[0.03] pb-2">
              <dt class="text-sm text-[var(--ir-text-muted)]">Public URI</dt>
              <dd class="font-mono text-sm">{{ status?.uri }}</dd>
            </div>
            <div class="flex justify-between border-b border-white/[0.03] pb-2">
              <dt class="text-sm text-[var(--ir-text-muted)]">CLI Binary</dt>
              <dd class="font-mono text-sm truncate max-w-[200px]" :title="status?.binary_resolved ?? ''">
                {{ status?.binary_resolved?.split('\\').pop()?.split('/').pop() ?? '—' }}
              </dd>
            </div>
            <div class="flex justify-between">
              <dt class="text-sm text-[var(--ir-text-muted)]">Started At</dt>
              <dd class="text-sm">{{ status?.started_at ? new Date(status.started_at).toLocaleString() : '—' }}</dd>
            </div>
          </dl>
        </div>

        <!-- Quick Actions -->
        <div class="glass-pane p-5">
          <h2 class="mb-4 text-sm font-semibold uppercase tracking-[0.08em]">Actions</h2>
          <div class="grid gap-3 sm:grid-cols-2">
            <button
              class="btn-secondary flex items-center justify-center gap-2"
              @click="handleRestart"
            >
              <ArrowPathIcon class="h-4 w-4" />
              Restart Server
            </button>
            <button
              class="btn-primary flex items-center justify-center gap-2"
              @click="handlePublish"
            >
              <ArrowPathIcon class="h-4 w-4" />
              Publish Module
            </button>
          </div>
          <p v-if="status?.last_error" class="mt-4 rounded border border-[var(--ir-danger)]/20 bg-[var(--ir-danger)]/5 p-3 text-xs text-[var(--ir-danger)]">
            <span class="font-bold block mb-1 uppercase tracking-wider">Last Error:</span>
            {{ status.last_error }}
          </p>
        </div>
      </section>
    </template>
  </div>
</template>
