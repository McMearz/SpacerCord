<script setup lang="ts">
import {
  CircleStackIcon,
  TableCellsIcon,
  CodeBracketIcon,
  CommandLineIcon,
  DocumentTextIcon,
  SignalIcon,
  PlayIcon,
  TrashIcon,
} from '@heroicons/vue/24/outline';
import { useSpacetimedb } from '~/composables/useSpacetimedb';

const { executeSql } = useSpacetimedb();
const { push } = useToast();

const query = ref('SELECT * FROM player_profile LIMIT 10');
const results = ref<any[]>([]);
const loading = ref(false);
const error = ref<string | null>(null);

async function runQuery() {
  if (!query.value.trim()) return;
  loading.value = true;
  error.value = null;
  try {
    const res = await executeSql(query.value);
    results.value = Array.isArray(res.data) ? res.data : [res.data];
    push({ type: 'success', title: 'Query executed' });
  } catch (e: any) {
    error.value = e.message;
    push({ type: 'error', title: 'Query failed', message: e.message });
  } finally {
    loading.value = false;
  }
}

const columns = computed(() => {
  if (results.value.length === 0) return [];
  return Object.keys(results.value[0]);
});

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
  <div class="flex h-full flex-col gap-5">
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

    <!-- SQL Editor -->
    <div class="glass-pane flex flex-col overflow-hidden">
      <div class="flex items-center justify-between border-b border-white/[0.03] bg-white/[0.02] p-3 px-4">
        <div class="flex items-center gap-2">
          <CommandLineIcon class="h-4 w-4 text-[var(--ir-accent)]" />
          <h2 class="text-xs font-bold uppercase tracking-widest text-[var(--ir-text-muted)]">SQL Console</h2>
        </div>
        <div class="flex gap-2">
          <button @click="results = []; error = null" class="btn-ghost btn-xs">
            <TrashIcon class="h-3.5 w-3.5" />
            Clear
          </button>
          <button
            class="btn-primary btn-xs flex items-center gap-1.5 px-4"
            :disabled="loading || !query.trim()"
            @click="runQuery"
          >
            <PlayIcon class="h-3.5 w-3.5" />
            Run Query
          </button>
        </div>
      </div>
      <div class="relative flex-1 min-h-[200px]">
        <textarea
          v-model="query"
          class="h-full w-full bg-transparent p-4 font-mono text-sm outline-none resize-none text-orange-100/90"
          spellcheck="false"
          placeholder="Enter SQL query..."
        />
        <div v-if="loading" class="absolute inset-0 bg-[var(--ir-bg-deep)]/50 backdrop-blur-sm flex items-center justify-center">
           <ArrowPathIcon class="h-8 w-8 animate-spin text-[var(--ir-accent)]" />
        </div>
      </div>
    </div>

    <!-- Error State -->
    <div v-if="error" class="glass-pane border-[var(--ir-danger)]/30 bg-[var(--ir-danger)]/5 p-4 text-sm text-[var(--ir-danger)] font-mono whitespace-pre-wrap">
      {{ error }}
    </div>

    <!-- Results -->
    <div class="glass-pane flex-1 overflow-hidden flex flex-col">
      <div class="border-b border-white/[0.03] bg-white/[0.01] p-2 px-4 flex justify-between items-center">
        <span class="text-[10px] font-bold uppercase tracking-widest text-[var(--ir-text-muted)]">Results</span>
        <span v-if="results.length > 0" class="text-[10px] font-mono text-[var(--ir-text-muted)]">{{ results.length }} rows returned</span>
      </div>
      <div class="flex-1 overflow-auto">
        <table v-if="results.length > 0" class="w-full border-collapse text-left text-sm">
          <thead class="sticky top-0 z-10 bg-[var(--ir-surface)] shadow-sm">
            <tr>
              <th
                v-for="col in columns"
                :key="col"
                class="px-4 py-2 font-mono text-[10px] uppercase tracking-widest text-[var(--ir-text-muted)] border-b border-white/[0.05]"
              >
                {{ col }}
              </th>
            </tr>
          </thead>
          <tbody class="divide-y divide-white/[0.03]">
            <tr
              v-for="(row, i) in results"
              :key="i"
              class="hover:bg-white/[0.01] transition-colors"
            >
              <td
                v-for="col in columns"
                :key="col"
                class="px-4 py-2 font-mono text-[11px] whitespace-nowrap overflow-hidden text-ellipsis max-w-[300px]"
                :title="JSON.stringify(row[col])"
              >
                {{ typeof row[col] === 'object' ? JSON.stringify(row[col]) : row[col] }}
              </td>
            </tr>
          </tbody>
        </table>
        <div v-else-if="!loading" class="flex h-full items-center justify-center p-12 text-[var(--ir-text-muted)] italic text-sm">
          No results to display. Run a query to see data.
        </div>
      </div>
    </div>
  </div>
</template>
