/**
 * Storage utilities for managing private keys in localStorage
 */

const PRIVATE_KEY_PREFIX = 'mailbox-private-key-';

export interface StorageKeys {
  privateKey: string;
}

/**
 * Get the storage key for a mailbox's private key
 */
export function getPrivateKeyStorageKey(publicKey: string): string {
  return PRIVATE_KEY_PREFIX + publicKey;
}

/**
 * Save a private key to localStorage
 */
export function savePrivateKey(publicKey: string, privateKey: string): void {
  localStorage.setItem(getPrivateKeyStorageKey(publicKey), privateKey);
}

/**
 * Get a private key from localStorage
 */
export function getPrivateKey(publicKey: string): string | null {
  return localStorage.getItem(getPrivateKeyStorageKey(publicKey));
}

/**
 * Remove a private key from localStorage
 */
export function removePrivateKey(publicKey: string): void {
  localStorage.removeItem(getPrivateKeyStorageKey(publicKey));
}

/**
 * Check if a private key exists in localStorage
 */
export function hasPrivateKey(publicKey: string): boolean {
  return getPrivateKey(publicKey) !== null;
} 