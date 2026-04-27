import type { StdbStatusDto, ApiEnvelope } from '~/types/api';

export const useSpacetimedb = () => {
  const { request } = useApi();
  const { status: connectionStatus } = useEventBus();

  const status = ref<StdbStatusDto | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);

  const fetchStatus = async () => {
    loading.value = true;
    try {
      const response = await request<ApiEnvelope<StdbStatusDto>>('/spacetimedb/status');
      status.value = response.data;
      error.value = null;
    } catch (e: any) {
      error.value = e.message || 'Failed to fetch SpacetimeDB status';
    } finally {
      loading.value = false;
    }
  };

  const restart = async () => {
    await request('/spacetimedb/restart', { method: 'POST', body: {} });
    await fetchStatus();
  };

  const publish = async () => {
    await request('/spacetimedb/publish', { method: 'POST', body: {} });
    await fetchStatus();
  };

  const executeSql = async (query: string) => {
    return await request<ApiEnvelope<any>>('/spacetimedb/sql', { 
        method: 'POST', 
        body: { query } 
    });
  };

  const getTableRows = async (tableName: string, limit = 100, offset = 0) => {
    return await request<ApiEnvelope<any[]>>(`/spacetimedb/tables/${tableName}/rows`, { 
        query: { limit, offset } 
    });
  };

  const getSchema = async () => {
    return await request<ApiEnvelope<any>>('/spacetimedb/schema');
  };

  // Auto-refresh status when connection opens
  watch(connectionStatus, (newStatus) => {
    if (newStatus === 'open') {
      fetchStatus();
    }
  }, { immediate: true });

  return {
    status,
    loading,
    error,
    fetchStatus,
    restart,
    publish,
    executeSql,
    getTableRows,
    getSchema
  };
};
