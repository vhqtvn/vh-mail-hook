declare module 'bech32' {
  interface Bech32Interface {
    encode(prefix: string, words: number[]): string;
    decode(str: string): { prefix: string; words: number[] };
    toWords(bytes: number[] | Uint8Array): number[];
    fromWords(words: number[]): number[];
  }

  export const bech32: Bech32Interface;
  export const bech32m: Bech32Interface;
} 