// We'll use dynamic imports for both modules
import type { bech32 } from 'bech32';

export interface AgeKeyPair {
  publicKey: string;
  privateKey: string;
}

// Convert Uint8Array to Base64 string
function uint8ArrayToBase64(bytes: Uint8Array): string {
  let binary = '';
  for (let i = 0; i < bytes.byteLength; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  return btoa(binary);
}

export async function validateAgePublicKey(key: string): Promise<boolean> {
  try {
    if (!key.startsWith('age1')) return false;
    const bech32Module = await import('bech32');
    const decoded = bech32Module.bech32.decode(key);
    return decoded.prefix === 'age' && decoded.words.length === 32; // X25519 public key is 32 bytes
  } catch {
    return false;
  }
}

export async function generateAgeKeyPair(): Promise<AgeKeyPair> {
  // Import modules dynamically
  const [naclModule, bech32Module] = await Promise.all([
    import('tweetnacl'),
    import('bech32')
  ]);
  
  // Generate X25519 key pair using TweetNaCl
  const keyPair = naclModule.box.keyPair();
  
  // Convert public key to 5-bit words for bech32
  const publicKeyWords = bech32Module.bech32.toWords(Array.from(keyPair.publicKey));
  
  // Encode public key in bech32 format with 'age' prefix
  const publicKey = bech32Module.bech32.encode('age', publicKeyWords);
  
  const privateKeyWords = bech32Module.bech32.toWords(Array.from(keyPair.secretKey));
  const privateKey = bech32Module.bech32.encode('AGE-SECRET-KEY-', privateKeyWords).toLocaleUpperCase();

  return { publicKey, privateKey };
} 