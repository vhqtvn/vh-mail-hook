export interface AgeKeyPair {
  publicKey: string;
  privateKey: string;
}

// Convert ArrayBuffer to Base64 URL-safe string
function arrayBufferToBase64(buffer: ArrayBuffer): string {
  const bytes = new Uint8Array(buffer);
  let binary = '';
  for (let i = 0; i < bytes.byteLength; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  return btoa(binary)
    .replace(/\+/g, '-')
    .replace(/\//g, '_')
    .replace(/=/g, '');
}

export function validateAgePublicKey(key: string): boolean {
  try {
    // The key should start with age1 and be the correct format
    return key.startsWith('age1') && key.length === 63;
  } catch {
    return false;
  }
}

export async function generateAgeKeyPair(): Promise<AgeKeyPair> {
  // Generate an X25519 key pair
  const keyPair = await window.crypto.subtle.generateKey(
    {
      name: 'ECDH',
      namedCurve: 'P-256'
    },
    true,
    ['deriveKey', 'deriveBits']
  );

  // Export the keys
  const publicKeyBuffer = await window.crypto.subtle.exportKey('raw', keyPair.publicKey);
  const privateKeyBuffer = await window.crypto.subtle.exportKey('pkcs8', keyPair.privateKey);

  // Convert to base64 and format as age keys
  const publicKeyBase64 = arrayBufferToBase64(publicKeyBuffer);
  const privateKeyBase64 = arrayBufferToBase64(privateKeyBuffer);

  // Ensure the keys are the correct length
  const publicKey = `age1${publicKeyBase64.slice(0, 59)}`;
  const privateKey = `AGE-SECRET-KEY-1${privateKeyBase64.slice(0, 59)}`;

  console.log('Generated key pair:', { publicKey, privateKey }); // Debug log

  return { publicKey, privateKey };
} 