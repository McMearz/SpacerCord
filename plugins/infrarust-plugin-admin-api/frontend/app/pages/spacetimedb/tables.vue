<script setup lang="ts">
import {
  CircleStackIcon,
  TableCellsIcon,
  CodeBracketIcon,
  CommandLineIcon,
  DocumentTextIcon,
  SignalIcon,
  MagnifyingGlassIcon,
  ArrowPathIcon,
} from '@heroicons/vue/24/outline';
import { useSpacetimedb } from '~/composables/useSpacetimedb';

const { getSchema, getTableRows } = useSpacetimedb();
const { push } = useToast();

const schema = ref<any>(null);
const selectedTable = ref<string | null>(null);
const rows = ref<any[]>([]);
const loadingRows = ref(false);
const searchQuery = ref('');

const tables = computed(() => {
  if (!schema.value?.tables) return [];
  return schema.value.tables.map((t: any) => t.table_name);
});

async function loadSchema() {
  try {
    const res = await getSchema();
    schema.value = res.data;
    if (tables.value.length > 0 && !selectedTable.value) {
      selectedTable.value = tables.value[0];
    }
  } catch (e: any) {
    push({ type: 'error', title: 'Failed to load schema', message: e.message });
  }
}

async function loadRows() {
  if (!selectedTable.value) return;
  loadingRows.value = true;
  try {
    const res = await getTableRows(selectedTable.value);
    rows.value = res.data;
  } catch (e: any) {
    push({ type: 'error', title: `Failed to load rows for ${selectedTable.value}`, message: e.message });
  } finally {
    loadingRows.value = false;
  }
}

watch(selectedTable, () => {
  loadRows();
});

onMounted(() => {
  loadSchema();
});

const subNav = [
  { to: '/spacetimedb', label: 'Overview', icon: CircleStackIcon },
  { to: '/spacetimedb/tables', label: 'Tables', icon: TableCellsIcon },
  { to: '/spacetimedb/reducers', label: 'Reducers', icon: CodeBracketIcon },
  { to: '/spacetimedb/sql', label: 'SQL Console', icon: CommandLineIcon },
  { to: '/spacetimedb/events', label: 'Event Feed', icon: SignalIcon },
  { to: '/spacetimedb/console', label: 'Live Logs', icon: DocumentTextIcon },
];

const filteredRows = computed(() => {
  if (!searchQuery.value) return rows.value;
  const q = searchQuery.value.toLowerCase();
  return rows.value.filter((row) => 
    JSON.stringify(row).toLowerCase().includes(q)
  );
});

const columns = computed(() => {
  if (rows.value.length === 0) return [];
  return Object.keys(rows.value[0]);
});
</script>

<template>
  <div class="grid gap-5 h-full flex-col">
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

    <div class="flex gap-4 min-h-0 flex-1 overflow-hidden">
      <!-- Sidebar: Tables -->
      <aside class="glass-pane w-64 flex flex-col shrink-0 overflow-hidden">
        <div class="p-4 border-b border-white/[0.03] flex items-center justify-between">
          <h2 class="text-xs font-semibold uppercase tracking-widest text-[var(--ir-text-muted)]">Tables</h2>
          <button @click="loadSchema" class="text-[var(--ir-text-muted)] hover:text-white transition-colors">
            <ArrowPathIcon class="h-3.5 w-3.5" />
          </button>
        </div>
        <nav class="flex-1 overflow-y-auto p-2 space-y-1">
          <button
            v-for="table in tables"
            :key="table"
            @click="selectedTable = table"
            class="w-full text-left px-3 py-2 rounded-md text-sm transition-colors"
            :class="selectedTable === table ? 'bg-[var(--ir-accent-soft)] text-white' : 'text-[var(--ir-text-muted)] hover:bg-white/[0.02] hover:text-white'"
          >
            {{ table }}
          </button>
          <div v-if="tables.length === 0" class="p-4 text-center text-xs text-[var(--ir-text-muted)]">
            No tables found.
          </div>
        </nav>
      </aside>

      <!-- Main: Data -->
      <main class="glass-pane flex-1 flex flex-col overflow-hidden">
        <header class="p-4 border-b border-white/[0.03] flex items-center justify-between gap-4">
          <div class="flex items-center gap-2">
            <h2 class="font-bold text-lg">{{ selectedTable ?? 'Select a Table' }}</h2>
            <span v-if="rows.length > 0" class="text-xs text-[var(--ir-text-muted)] font-mono">
              {{ rows.length }} rows
            </span>
          </div>
          
          <div class="relative flex-1 max-w-sm">
            <MagnifyingGlassIcon class="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-[var(--ir-text-muted)]" />
            <input
              v-model="searchQuery"
              class="input pl-9 w-full py-1.5 text-sm"
              placeholder="Search rows..."
            />
          </div>

          <button @click="loadRows" class="btn-ghost btn-xs p-1" :disabled="loadingRows">
            <ArrowPathIcon class="h-4 w-4" :class="{ 'animate-spin': loadingRows }" />
          </button>
        </header>

        <div class="flex-1 overflow-auto">
          <table class="w-full border-collapse text-left text-sm">
            <thead class="sticky top-0 z-10 bg-[var(--ir-surface)]/95 backdrop-blur shadow-sm">
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
                v-for="(row, i) in filteredRows"
                :key="i"
                class="hover:bg-white/[0.01] transition-colors"
              >
                <td
                  v-for="col in columns"
                  :key="col"
                  class="px-4 py-2 font-mono text-[11px] whitespace-nowrap overflow-hidden text-ellipsis max-w-[200px]"
                  :title="JSON.stringify(row[col])"
                >
                  <span :class="typeof row[col] === 'string' ? 'text-green-400' : 'text-orange-400'">
                    {{ typeof row[col] === 'object' ? JSON.stringify(row[col]) : row[col] }}
                  </span>
                </td>
              </tr>
              <tr v-if="filteredRows.length === 0">
                <td :colspan="columns.length || 1" class="px-4 py-12 text-center text-[var(--ir-text-muted)] italic">
                  {{ loadingRows ? 'Loading rows...' : 'No data found in this table.' }}
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </main>
    </div>
  </div>
</template>
