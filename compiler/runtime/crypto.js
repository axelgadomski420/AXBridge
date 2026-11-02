const enc=new TextEncoder(),dec=new TextDecoder();
export const toB64=b=>btoa(String.fromCharCode(...new Uint8Array(b)));
export const fromB64=s=>Uint8Array.from(atob(s),c=>c.charCodeAt(0));
export async function sha256(v){return toB64(await crypto.subtle.digest("SHA-256",typeof v==="string"?enc.encode(v):v));}
export async function identity(){const pair=await crypto.subtle.generateKey({name:"ECDH",namedCurve:"P-256"},true,["deriveKey"]);return{publicKey:await crypto.subtle.exportKey("jwk",pair.publicKey),privateKey:await crypto.subtle.exportKey("jwk",pair.privateKey)};}
export async function safetyNumber(a,b){const values=[a,b].sort().join(":");const hash=new Uint8Array(await crypto.subtle.digest("SHA-256",enc.encode(values)));return Array.from({length:12},(_,i)=>((hash[i*2]<<8)|hash[i*2+1]).toString().padStart(5,"0").slice(0,5)).join(" ");}
export class Ratchet{
  constructor(secret,counter=0){this.secret=secret;this.counter=counter;}
  static async create(root){return new Ratchet(fromB64(root),0);}
  async next(){const next=new Uint8Array(await crypto.subtle.digest("SHA-256",this.secret));this.secret=next;this.counter++;return crypto.subtle.importKey("raw",next,"AES-GCM",false,["encrypt","decrypt"]);}
  async encrypt(plaintext,aad){const iv=crypto.getRandomValues(new Uint8Array(12)),key=await this.next();const ciphertext=await crypto.subtle.encrypt({name:"AES-GCM",iv,additionalData:enc.encode(aad)},key,enc.encode(plaintext));return{ciphertext:toB64(ciphertext),nonce:toB64(iv),counter:this.counter};}
  async decrypt(packet,aad){if(packet.counter!==this.counter+1)throw new Error("Replay or out-of-order message");const key=await this.next();return dec.decode(await crypto.subtle.decrypt({name:"AES-GCM",iv:fromB64(packet.nonce),additionalData:enc.encode(aad)},key,fromB64(packet.ciphertext)));}
  export(){return{secret:toB64(this.secret),counter:this.counter};}
}
export async function sealWithPassphrase(value,passphrase){const salt=crypto.getRandomValues(new Uint8Array(16)),iv=crypto.getRandomValues(new Uint8Array(12));const material=await crypto.subtle.importKey("raw",enc.encode(passphrase),"PBKDF2",false,["deriveKey"]);const key=await crypto.subtle.deriveKey({name:"PBKDF2",salt,iterations:210000,hash:"SHA-256"},material,{name:"AES-GCM",length:256},false,["encrypt"]);const ciphertext=await crypto.subtle.encrypt({name:"AES-GCM",iv},key,enc.encode(JSON.stringify(value)));return{version:1,kdf:"PBKDF2-SHA256",iterations:210000,salt:toB64(salt),nonce:toB64(iv),ciphertext:toB64(ciphertext)};}
export async function openWithPassphrase(packet,passphrase){const material=await crypto.subtle.importKey("raw",enc.encode(passphrase),"PBKDF2",false,["deriveKey"]);const key=await crypto.subtle.deriveKey({name:"PBKDF2",salt:fromB64(packet.salt),iterations:packet.iterations,hash:"SHA-256"},material,{name:"AES-GCM",length:256},false,["decrypt"]);return JSON.parse(dec.decode(await crypto.subtle.decrypt({name:"AES-GCM",iv:fromB64(packet.nonce)},key,fromB64(packet.ciphertext))));}
export async function backup(value,passphrase){return JSON.stringify(await sealWithPassphrase(value,passphrase));}
export async function restore(value,passphrase){return openWithPassphrase(JSON.parse(value),passphrase);}
