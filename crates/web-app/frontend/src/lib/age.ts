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
    console.log(decoded);
    return decoded.prefix === 'age' && decoded.words.length === 52;
  } catch {
    return false;
  }
}

// this help inconsistence between dev and prod
var __importDefault = function (mod: any) {
  return Object.keys(mod).length === 1 && mod.default ? mod.default : mod;
};

export async function generateAgeKeyPair(): Promise<AgeKeyPair> {
  const [naclModule, bech32Module] = (await Promise.all([
    import('tweetnacl'),
    import('bech32')
  ])).map(m => __importDefault(m));

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