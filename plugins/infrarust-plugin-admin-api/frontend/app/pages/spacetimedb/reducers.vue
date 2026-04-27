<script setup lang="ts">
import {
  CircleStackIcon,
  TableCellsIcon,
  CodeBracketIcon,
  CommandLineIcon,
  DocumentTextIcon,
  SignalIcon,
  PlayIcon,
} from '@heroicons/vue/24/outline';
import { useSpacetimedb } from '~/composables/useSpacetimedb';

const { getSchema } = useSpacetimedb();
const { request } = useApi();
const { push } = useToast();

const schema = ref<any>(null);
const loading = ref(false);
const callingReducer = ref<string | null>(null);

const reducers = computed(() => {
  if (!schema.value?.reducers) return [];
  return schema.value.reducers.map((r: any) => ({
    name: r.reducer_name,
    args: r.args || []
  }));
});

async function loadSchema() {
  loading.value = true;
  try {
    const res = await getSchema();
    schema.value = res.data;
  } catch (e: any) {
    push({ type: 'error', title: 'Failed to load schema', message: e.message });
  } finally {
    loading.value = false;
  }
}

const reducerArgs = ref<Record<string, string>>({});

async function handleCall(reducerName: string) {
  callingReducer.value = reducerName;
  try {
    const rawArgs = reducerArgs.value[reducerName] || '{}';
    let parsedArgs;
    try {
      parsedArgs = JSON.parse(rawArgs);
    } catch {
      push({ type: 'error', title: 'Invalid JSON', message: 'Arguments must be valid JSON' });
      return;
    }

    await request(`/spacetimedb/reducers/${reducerName}`, {
      method: 'POST',
      body: { args: parsedArgs }
    });
    push({ type: 'success', title: `Reducer ${reducerName} called` });
  } catch (e: any) {
    push({ type: 'error', title: `Failed to call ${reducerName}`, message: e.message });
  } finally {
    callingReducer.value = null;
  }
}

onMounted(loadSchema);

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

    <div class="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
      <div
        v-for="reducer in reducers"
        :key="reducer.name"
        class="glass-pane p-5 flex flex-col"
      >
        <div class="flex items-center justify-between mb-4">
          <div class="flex items-center gap-2">
            <CodeBracketIcon class="h-4 w-4 text-[var(--ir-accent)]" />
            <h3 class="font-bold">{{ reducer.name }}</h3>
          </div>
          <span class="text-[10px] font-mono text-[var(--ir-text-muted)] uppercase tracking-wider">Reducer</span>
        </div>

        <div class="flex-1">
          <label class="block text-[10px] font-bold uppercase tracking-widest text-[var(--ir-text-muted)] mb-1.5">Arguments (JSON)</label>
          <textarea
            v-model="reducerArgs[reducer.name]"
            class="input w-full font-mono text-xs h-24 resize-none bg-black/20"
            placeholder='{ "uuid": "...", "username": "..." }'
          />
        </div>

        <button
          class="btn-primary w-full mt-4 flex items-center justify-center gap-2"
          :disabled="callingReducer === reducer.name"
          @click="handleCall(reducer.name)"
        >
          <ArrowPathIcon v-if="callingReducer === reducer.name" class="h-4 w-4 animate-spin" />
          <PlayIcon v-else class="h-4 w-4" />
          Execute
        </button>
      </div>

      <div v-if="reducers.length === 0 && !loading" class="col-span-full py-12 text-center text-[var(--ir-text-muted)] italic">
        No reducers found in module schema.
      </div>
      
      <div v-if="loading" class="col-span-full py-12 text-center">
        <ArrowPathIcon class="h-8 w-8 animate-spin mx-auto text-[var(--ir-accent)] opacity-50" />
        <p class="mt-2 text-sm text-[var(--ir-text-muted)]">Loading schema...</p>
      </div>
    </div>
  </div>
</template>
