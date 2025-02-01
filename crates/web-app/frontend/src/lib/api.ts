export interface FetchOptions extends RequestInit {
  requireAuth?: boolean;
}

export async function fetchApi(endpoint: string, options: FetchOptions = {}) {
  const { requireAuth = true, headers = {}, ...rest } = options;

  const defaultHeaders: Record<string, string> = {
    'Content-Type': 'application/json',
  };

  if (requireAuth) {
    // The server will use the session cookie for authentication
    // We set this header to indicate that this is an authenticated request
    defaultHeaders['X-Requested-With'] = 'XMLHttpRequest';
  }

  const response = await fetch(endpoint, {
    ...rest,
    headers: {
      ...defaultHeaders,
      ...headers,
    },
    credentials: 'same-origin', // This is important for sending cookies
  });

  return response;
}

export async function get(endpoint: string, options: FetchOptions = {}) {
  return fetchApi(endpoint, { ...options, method: 'GET' });
}

export async function post(endpoint: string, data: any, options: FetchOptions = {}) {
  return fetchApi(endpoint, {
    ...options,
    method: 'POST',
    body: JSON.stringify(data),
  });
}

export async function del(endpoint: string, options: FetchOptions = {}) {
  return fetchApi(endpoint, { ...options, method: 'DELETE' });
} 