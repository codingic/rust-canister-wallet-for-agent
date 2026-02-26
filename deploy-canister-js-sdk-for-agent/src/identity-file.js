import { readFile, writeFile } from "node:fs/promises";
import { randomBytes, scryptSync, createCipheriv, createDecipheriv } from "node:crypto";
import { Ed25519KeyIdentity } from "@dfinity/identity";

const HEADER = "-----BEGIN OPENCLAW ENCRYPTED IDENTITY-----";
const FOOTER = "-----END OPENCLAW ENCRYPTED IDENTITY-----";
const FORMAT_VERSION = 1;

function ensurePassword(password) {
  if (typeof password !== "string" || password.length < 8) {
    throw new Error("password is required and must be at least 8 characters");
  }
}

function armorPayload(jsonText) {
  const base64 = Buffer.from(jsonText, "utf8").toString("base64");
  const lines = base64.match(/.{1,64}/g) ?? [];
  return [HEADER, ...lines, FOOTER, ""].join("\n");
}

function unarmorPayload(text) {
  const trimmed = text.trim();
  if (!trimmed.startsWith(HEADER) || !trimmed.endsWith(FOOTER)) {
    throw new Error("invalid encrypted identity file format");
  }
  const base64 = trimmed
    .slice(HEADER.length, trimmed.length - FOOTER.length)
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean)
    .join("");
  return Buffer.from(base64, "base64").toString("utf8");
}

function encryptJson(plaintextJson, password) {
  ensurePassword(password);
  const salt = randomBytes(16);
  const iv = randomBytes(12);
  const key = scryptSync(password, salt, 32);
  const cipher = createCipheriv("aes-256-gcm", key, iv);
  const ciphertext = Buffer.concat([
    cipher.update(Buffer.from(plaintextJson, "utf8")),
    cipher.final(),
  ]);
  const authTag = cipher.getAuthTag();

  return JSON.stringify({
    v: FORMAT_VERSION,
    kdf: "scrypt",
    cipher: "aes-256-gcm",
    salt: salt.toString("base64"),
    iv: iv.toString("base64"),
    tag: authTag.toString("base64"),
    data: ciphertext.toString("base64"),
  });
}

function decryptJson(encryptedEnvelopeJson, password) {
  ensurePassword(password);
  const envelope = JSON.parse(encryptedEnvelopeJson);
  if (envelope?.v !== FORMAT_VERSION) {
    throw new Error(`unsupported identity file version: ${envelope?.v}`);
  }
  if (envelope?.kdf !== "scrypt" || envelope?.cipher !== "aes-256-gcm") {
    throw new Error("unsupported identity file encryption parameters");
  }
  const salt = Buffer.from(envelope.salt, "base64");
  const iv = Buffer.from(envelope.iv, "base64");
  const tag = Buffer.from(envelope.tag, "base64");
  const data = Buffer.from(envelope.data, "base64");
  const key = scryptSync(password, salt, 32);
  const decipher = createDecipheriv("aes-256-gcm", key, iv);
  decipher.setAuthTag(tag);
  const plaintext = Buffer.concat([decipher.update(data), decipher.final()]);
  return plaintext.toString("utf8");
}

export async function createEncryptedEd25519IdentityFile({
  filePath,
  password,
  overwrite = false,
}) {
  ensurePassword(password);
  const identity = Ed25519KeyIdentity.generate();
  const secretJson = JSON.stringify(identity.toJSON());
  const encrypted = encryptJson(secretJson, password);
  const armored = armorPayload(encrypted);
  const principal = identity.getPrincipal().toText();
  await writeFile(filePath, armored, {
    encoding: "utf8",
    flag: overwrite ? "w" : "wx",
    mode: 0o600,
  });
  return { identity, principal, filePath };
}

export async function loadEncryptedEd25519IdentityFile({ filePath, password }) {
  ensurePassword(password);
  const text = await readFile(filePath, "utf8");
  const encryptedJson = unarmorPayload(text);
  const decryptedJson = decryptJson(encryptedJson, password);
  const identity = Ed25519KeyIdentity.fromJSON(decryptedJson);
  const principal = identity.getPrincipal().toText();
  return { identity, principal, filePath };
}

export async function fileExists(filePath) {
  try {
    await readFile(filePath);
    return true;
  } catch {
    return false;
  }
}

