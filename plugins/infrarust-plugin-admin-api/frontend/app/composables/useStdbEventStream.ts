import type { StdbRowEventDto } from '~/types/api';

export const useStdbEventStream = () => {
  const config = useRuntimeConfig();
  const { apiKey } = useAuth();

  const events = ref<StdbRowEventDto[]>([]);
  const status = ref<'idle' | 'connecting' | 'open' | 'closed' | 'error'>('idle');
  const paused = ref(false);

  let eventSource: EventSource | null = null;
  let reconnectTimer: ReturnType<typeof setTimeout> | null = null;

  function buildUrl() {
    const url = new URL(`${config.public.apiBase}/spacetimedb/events`, window.location.origin);
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

    eventSource.addEventListener('stdb-event', (event: MessageEvent) => {
      if (paused.value) return;
      try {
        const parsed: StdbRowEventDto = JSON.parse(event.data);
        events.value = [parsed, ...events.value].slice(0, 100);
      } catch {
        // Malformed — skip
      }
    });
  }

  function togglePause() {
    if (paused.value) {
      paused.value = false;
      connect();
    } else {
      paused.value = true;
      close();
    }
  }

  onMounted(() => connect());
  onBeforeUnmount(close);

  return {
    events,
    status: readonly(status),
    paused: readonly(paused),
    togglePause,
    clear: () => { events.value = [] },
  };
};
