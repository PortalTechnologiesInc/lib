/**
 * Error codes for programmatic handling.
 */
export type PortalSDKErrorCode =
  | 'NOT_CONNECTED'
  | 'CONNECTION_TIMEOUT'
  | 'CONNECTION_CLOSED'
  | 'AUTH_FAILED'
  | 'UNEXPECTED_RESPONSE'
  | 'SERVER_ERROR'
  | 'PARSE_ERROR';

/**
 * SDK error with optional code and context for production handling.
 */
export class PortalSDKError extends Error {
  readonly code: PortalSDKErrorCode;
  readonly details?: unknown;

  constructor(
    message: string,
    code: PortalSDKErrorCode = 'SERVER_ERROR',
    details?: unknown
  ) {
    super(message);
    this.name = 'PortalSDKError';
    this.code = code;
    this.details = details;
    Object.setPrototypeOf(this, PortalSDKError.prototype);
  }
}
