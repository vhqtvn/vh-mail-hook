export interface FetchOptions extends RequestInit {
  requireAuth?: boolean;
}

// Function to get the JWT token from localStorage
function getAuthToken(): string | null {
  return localStorage.getItem('auth_token');
}

// Function to set the JWT token in localStorage
export function setAuthToken(token: string): void {
  localStorage.setItem('auth_token', token);
}

// Function to remove the JWT token from localStorage
export function removeAuthToken(): void {
  localStorage.removeItem('auth_token');
}

export async function fetchApi(endpoint: string, options: FetchOptions = {}) {
  const { requireAuth = true, headers = {}, ...rest } = options;

  const defaultHeaders: Record<string, string> = {
    'Content-Type': 'application/json',
  };

  if (requireAuth) {
    const token = getAuthToken();
    if (token) {
      defaultHeaders['Authorization'] = `Bearer ${token}`;
    }
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