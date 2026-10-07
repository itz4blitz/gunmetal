import { enMessages, type EnMessages } from './en/index.ts';

export type MessageCatalogue = EnMessages;

// Function-returned table so mutants are covered per test (client plan, module-level data).
export function catalogue(): MessageCatalogue {
  return enMessages();
}
