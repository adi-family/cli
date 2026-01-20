/**
 * Shared HTTP client for all API calls.
 * 
 * Features:
 * - Automatic cookie-based authentication
 * - Consistent error handling
 * - Request/response interceptors
 * - Timeout handling
 */

// =============================================================================
// Types
// =============================================================================

/**
 * Configuration for the HTTP client.
 */
export interface ClientConfig {
  /** Base URL for all requests */
  baseUrl: string;
  
  /** Default timeout in milliseconds (default: 30000) */
  timeout?: number;
  
  /** Custom headers to include in every request */
  headers?: Record<string, string>;
  
  /** Whether to include credentials (cookies) in requests (default: true) */
  credentials?: RequestCredentials;
}

/**
 * Options for individual requests.
 */
export interface RequestOptions {
  /** Request body (will be JSON stringified) */
  body?: unknown;
  
  /** Query parameters */
  query?: Record<string, unknown>;
  
  /** Additional headers for this request */
  headers?: Record<string, string>;
  
  /** Override timeout for this request */
  timeout?: number;
  
  /** AbortSignal for cancellation */
  signal?: AbortSignal;
}

/**
 * API error with status code and optional error code.
 */
export class ApiError extends Error {
  readonly statusCode: number;
  readonly code: string;
  readonly details?: unknown;
  
  constructor(
    statusCode: number,
    code: string,
    message: string,
    details?: unknown
  ) {
    super(message);
    this.name = 'ApiError';
    this.statusCode = statusCode;
    this.code = code;
    this.details = details;
  }
  
  /** Check if error is a network/connectivity issue */
  get isNetworkError(): boolean {
    return this.statusCode === 0;
  }
  
  /** Check if error is an authentication issue */
  get isAuthError(): boolean {
    return this.statusCode === 401 || this.statusCode === 403;
  }
  
  /** Check if error is a not found issue */
  get isNotFound(): boolean {
    return this.statusCode === 404;
  }
  
  /** Check if error is a server issue (retriable) */
  get isServerError(): boolean {
    return this.statusCode >= 500;
  }
}

// =============================================================================
// HTTP Client
// =============================================================================

/**
 * Base HTTP client with consistent error handling and configuration.
 * Extend this class for specific API clients.
 */
export class HttpClient {
  private readonly baseUrl: string;
  private readonly timeout: number;
  private readonly defaultHeaders: Record<string, string>;
  private readonly credentials: RequestCredentials;
  
  constructor(config: ClientConfig) {
    // Remove trailing slash from base URL
    this.baseUrl = config.baseUrl.replace(/\/$/, '');
    this.timeout = config.timeout ?? 30000;
    this.defaultHeaders = {
      'Content-Type': 'application/json',
      ...config.headers,
    };
    this.credentials = config.credentials ?? 'include';
  }
  
  /**
   * Make an HTTP request.
   * 
   * @param method - HTTP method
   * @param path - Path relative to baseUrl
   * @param options - Request options
   * @returns Parsed JSON response
   */
  async request<T>(
    method: string,
    path: string,
    options: RequestOptions = {}
  ): Promise<T> {
    // Build URL with query parameters
    const url = this.buildUrl(path, options.query);
    
    // Merge headers
    const headers: Record<string, string> = {
      ...this.defaultHeaders,
      ...options.headers,
    };
    
    // Create abort controller for timeout
    const controller = new AbortController();
    const timeoutId = setTimeout(
      () => controller.abort(),
      options.timeout ?? this.timeout
    );
    
    // Combine with external signal if provided
    const signal = options.signal
      ? this.combineSignals(options.signal, controller.signal)
      : controller.signal;
    
    try {
      const response = await fetch(url.toString(), {
        method,
        headers,
        credentials: this.credentials,
        body: options.body ? JSON.stringify(options.body) : undefined,
        signal,
      });
      
      clearTimeout(timeoutId);
      
      // Handle non-OK responses
      if (!response.ok) {
        throw await this.createErrorFromResponse(response);
      }
      
      // Handle 204 No Content
      if (response.status === 204) {
        return undefined as T;
      }
      
      // Parse JSON response
      return await response.json();
    } catch (error) {
      clearTimeout(timeoutId);
      
      // Re-throw ApiError as-is
      if (error instanceof ApiError) {
        throw error;
      }
      
      // Handle abort/timeout
      if (error instanceof DOMException && error.name === 'AbortError') {
        throw new ApiError(0, 'TIMEOUT', 'Request timed out');
      }
      
      // Handle network errors
      if (error instanceof TypeError) {
        throw new ApiError(0, 'NETWORK_ERROR', 'Network error: ' + error.message);
      }
      
      // Re-throw unknown errors
      throw error;
    }
  }
  
  /**
   * Build URL with query parameters.
   */
  private buildUrl(path: string, query?: Record<string, unknown>): URL {
    // Handle absolute URLs
    const url = path.startsWith('http')
      ? new URL(path)
      : new URL(this.baseUrl + path, window.location.origin);
    
    if (query) {
      for (const [key, value] of Object.entries(query)) {
        if (value !== undefined && value !== null) {
          url.searchParams.set(key, String(value));
        }
      }
    }
    
    return url;
  }
  
  /**
   * Create an ApiError from a failed response.
   */
  private async createErrorFromResponse(response: Response): Promise<ApiError> {
    let code = 'ERROR';
    let message = response.statusText || `HTTP ${response.status}`;
    let details: unknown;
    
    try {
      const body = await response.json();
      code = body.code ?? body.error ?? code;
      message = body.message ?? body.error_description ?? message;
      details = body;
    } catch {
      // Response is not JSON, use status text
    }
    
    return new ApiError(response.status, code, message, details);
  }
  
  /**
   * Combine multiple AbortSignals into one.
   */
  private combineSignals(...signals: AbortSignal[]): AbortSignal {
    const controller = new AbortController();
    
    for (const signal of signals) {
      if (signal.aborted) {
        controller.abort();
        break;
      }
      signal.addEventListener('abort', () => controller.abort(), { once: true });
    }
    
    return controller.signal;
  }
  
  // ===========================================================================
  // Convenience Methods
  // ===========================================================================
  
  /** GET request */
  get<T>(path: string, options?: Omit<RequestOptions, 'body'>): Promise<T> {
    return this.request<T>('GET', path, options);
  }
  
  /** POST request */
  post<T>(path: string, body?: unknown, options?: Omit<RequestOptions, 'body'>): Promise<T> {
    return this.request<T>('POST', path, { ...options, body });
  }
  
  /** PUT request */
  put<T>(path: string, body?: unknown, options?: Omit<RequestOptions, 'body'>): Promise<T> {
    return this.request<T>('PUT', path, { ...options, body });
  }
  
  /** PATCH request */
  patch<T>(path: string, body?: unknown, options?: Omit<RequestOptions, 'body'>): Promise<T> {
    return this.request<T>('PATCH', path, { ...options, body });
  }
  
  /** DELETE request */
  delete<T>(path: string, options?: RequestOptions): Promise<T> {
    return this.request<T>('DELETE', path, options);
  }
}

// =============================================================================
// Utilities
// =============================================================================

/**
 * Check if the browser is currently online.
 */
export const isOnline = (): boolean => navigator.onLine;

/**
 * Create a promise that resolves when the browser comes online.
 */
export const waitForOnline = (): Promise<void> => {
  if (isOnline()) {
    return Promise.resolve();
  }
  
  return new Promise((resolve) => {
    const handler = () => {
      window.removeEventListener('online', handler);
      resolve();
    };
    window.addEventListener('online', handler);
  });
};
