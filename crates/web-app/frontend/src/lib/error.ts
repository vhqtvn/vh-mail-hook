import { ApiError } from './api';

export function getErrorMessage(error: unknown): string {
  if (error instanceof ApiError) {
    return error.message;
  }
  if (error instanceof Error) {
    return error.message;
  }
  return 'An unknown error occurred';
}

export function getDebugInfo(error: unknown): string {
  if (error instanceof ApiError) {
    return error.getDebugInfo();
  }
  if (error instanceof Error) {
    return error.stack || error.message;
  }
  return String(error);
}

export function isNetworkError(error: unknown): boolean {
  return error instanceof ApiError && error.status === 0;
}

export function shouldShowDebugInfo(error: unknown): boolean {
  // Show debug info for network errors or unexpected errors
  return isNetworkError(error) || !(error instanceof ApiError);
}

export function formatErrorMessage(error: unknown): { message: string; debug?: string } {
  const message = getErrorMessage(error);
  const shouldShowDebug = shouldShowDebugInfo(error);
  const debug = shouldShowDebug ? getDebugInfo(error) : undefined;

  return {
    message: shouldShowDebug
      ? 'An unexpected error occurred. Please try again or contact support if the problem persists.'
      : message,
    debug,
  };
} 