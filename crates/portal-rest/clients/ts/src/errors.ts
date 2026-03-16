/**
 * Error codes for programmatic handling.
 */
export type PortalSDKErrorCode =
  | 'HTTP_ERROR'
  | 'API_ERROR'
  | 'POLL_TIMEOUT'
  | 'PARSE_ERROR'
  | 'NETWORK_ERROR';

/**
 * SDK error with optional code and context for production handling.
 */
export class PortalSDKError extends Error {
  readonly code: PortalSDKErrorCode;
  readonly statusCode?: number;
  readonly details?: unknown;

  constructor(
    message: string,
    code: PortalSDKErrorCode = 'API_ERROR',
    details?: unknown,
    statusCode?: number
  ) {
    super(message);
    this.name = 'PortalSDKError';
    this.code = code;
    this.details = details;
    this.statusCode = statusCode;
    Object.setPrototypeOf(this, PortalSDKError.prototype);
  }
}
