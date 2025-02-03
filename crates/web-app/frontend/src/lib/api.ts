export interface FetchOptions extends RequestInit {
  requireAuth?: boolean;
}

export interface ApiResponse<T = any> {
  success: boolean;
  error?: string;
  data: T | null;
}

export class ApiError extends Error {
  constructor(
    message: string,
    public status: number,
    public endpoint: string,
    public requestDetails: {
      method: string;
      headers: Record<string, string>;
      body?: BodyInit | null;
    }
  ) {
    super(message);
    this.name = 'ApiError';
  }

  getDebugInfo(): string {
    return `
Request failed: ${this.message}
Status: ${this.status}
Endpoint: ${this.endpoint}
Method: ${this.requestDetails.method}
Headers: ${JSON.stringify(this.requestDetails.headers, null, 2)}
${this.requestDetails.body ? `Body: ${String(this.requestDetails.body)}` : ''}
    `.trim();
  }
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

export async function fetchApi<T = any>(endpoint: string, options: FetchOptions = {}): Promise<ApiResponse<T>> {
  const { requireAuth = true, headers = {}, ...rest } = options;

  const defaultHeaders: Record<string, string> = {
    'Content-Type': 'application/json',
    'Accept': 'application/json',
  };

  if (requireAuth) {
    const token = getAuthToken();
    if (token) {
      defaultHeaders['Authorization'] = `Bearer ${token}`;
    }
  }

  // Convert headers to a plain object to ensure type compatibility
  const mergedHeaders: Record<string, string> = {
    ...defaultHeaders,
    ...Object.fromEntries(
      Object.entries(headers).map(([key, value]) => [key, String(value)])
    ),
  };

  const requestDetails = {
    method: rest.method || 'GET',
    headers: mergedHeaders,
    body: rest.body,
  };

  try {
    const response = await fetch(endpoint, {
      ...rest,
      headers: mergedHeaders,
      credentials: 'same-origin',
    });

    const contentType = response.headers.get('content-type');
    const isJson = contentType && contentType.includes('application/json');

    if (!isJson) {
      throw new ApiError(
        'Unexpected response format',
        response.status,
        endpoint,
        requestDetails
      );
    }

    const data = await response.json();

    // If the response is not in our expected format, wrap it in our format
    if (!('success' in data)) {
      return {
        success: response.ok,
        error: response.ok ? undefined : 'Unknown error occurred',
        data: response.ok ? data : null,
      };
    }

    if (!response.ok || !data.success) {
      throw new ApiError(
        data.error || 'Unknown error occurred',
        response.status,
        endpoint,
        requestDetails
      );
    }

    return data as ApiResponse<T>;
  } catch (error) {
    if (error instanceof ApiError) {
      throw error;
    }

    // Handle network errors or other unexpected errors
    throw new ApiError(
      error instanceof Error ? error.message : 'Network error occurred',
      0, // Use 0 for network errors
      endpoint,
      requestDetails
    );
  }
}

export async function get<T = any>(endpoint: string, options: FetchOptions = {}): Promise<ApiResponse<T>> {
  return fetchApi<T>(endpoint, { ...options, method: 'GET' });
}

export async function post<T = any>(endpoint: string, data: any, options: FetchOptions = {}): Promise<ApiResponse<T>> {
  return fetchApi<T>(endpoint, {
    ...options,
    method: 'POST',
    body: JSON.stringify(data),
  });
}

export async function del<T = any>(endpoint: string, options: FetchOptions = {}): Promise<ApiResponse<T>> {
  return fetchApi<T>(endpoint, { ...options, method: 'DELETE' });
} 