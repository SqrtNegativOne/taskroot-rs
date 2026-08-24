export const BACKEND_ERROR_CODES = [
    'db',
    'not-found',
    'auth',
    'sync',
    'invalid-input',
    'not-ready',
    'internal',
] as const;

export type BackendErrorCode = (typeof BACKEND_ERROR_CODES)[number];

export type AppErrorCode = BackendErrorCode | 'unknown';

export interface AppError {
    code: AppErrorCode;
    message: string;
}

function isBackendCode(code: string): code is BackendErrorCode {
    return BACKEND_ERROR_CODES.some((backendCode) => backendCode === code);
}

interface SerializedErrorShape {
    code: unknown;
    message: unknown;
}

function hasSerializedErrorShape(value: object): value is SerializedErrorShape {
    return 'code' in value && 'message' in value;
}

export function normalizeAppError(error: unknown): AppError {
    if (typeof error === 'object' && error !== null && hasSerializedErrorShape(error)) {
        const { code, message } = error;
        return {
            code: typeof code === 'string' && isBackendCode(code) ? code : 'unknown',
            message: typeof message === 'string' ? message : String(message),
        };
    }
    return { code: 'unknown', message: error instanceof Error ? error.message : String(error) };
}

export function unknownAppError(message: string): AppError {
    return { code: 'unknown', message };
}

export function describeAppError(error: unknown): string {
    return normalizeAppError(error).message;
}
