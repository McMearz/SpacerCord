import type { LogEntryDto } from '~/types/api';

export const useStdbLogStream = () => {
  const config = useRuntimeConfig();
  const { apiKey } = useAuth();

  const logs = ref<LogEntryDto[]>([]);
  const status = ref<'idle' | 'connecting' | 'open' | 'closed' | 'error'>('idle');
  const paused = ref(false);

  let eventSource: EventSource | null = null;
  let reconnectTimer: ReturnType<typeof setTimeout> | null = null;

  function buildUrl() {
    const url = new URL(`${config.public.apiBase}/spacetimedb/logs`, window.location.origin);
    if (apiKey.value) url.searchParams.set('token', apiKey.value);
    return url.toString();
  }

  function close() {
    if (eventSource) {
      eventSource.close();
      eventSource = null;
    }
    if (reconnectTimer) {
      clearTimeout(reconnectTimer);
      reconnectTimer = null;
    }
    status.value = 'closed';
  }

  function connect() {
    if (!import.meta.client || !apiKey.value) return;

    close();
    status.value = 'connecting';
    paused.value = false;

    eventSource = new EventSource(buildUrl());

    eventSource.onopen = () => {
      status.value = 'open';
    };

    eventSource.onerror = () => {
      status.value = 'error';
      close();
      reconnectTimer = setTimeout(() => connect(), 2000);
    };

    eventSource.addEventListener('stdb-log', (event: MessageEvent) => {
      if (paused.value) return;
      try {
        const parsed = JSON.parse(event.data);
        const entry: LogEntryDto = {
            timestamp: parsed.timestamp,
            level: parsed.source.toLowerCase(), // Re-use level for source (Runtime, Install, etc.)
            target: parsed.source,
            message: parsed.message
        };
        logs.value = [...logs.value, entry].slice(-2000);
      } catch {
        // Malformed — skip
      }
    });
  }

  function pause() {
    paused.value = true;
    close();
  }

  function resume() {
    paused.value = false;
    connect();
  }

  function clear() {
    logs.value = [];
  }

  onMounted(() => connect());
  onBeforeUnmount(close);

  return {
    logs,
    status: readonly(status),
    paused: readonly(paused),
    connect,
    close,
    pause,
    resume,
    clear,
  };
};
