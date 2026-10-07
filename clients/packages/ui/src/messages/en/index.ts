import { destinationMessages, type DestinationMessages } from './destinations.ts';
import { shellMessages, type ShellMessages } from './shell.ts';

export type EnMessages = {
  shell: ShellMessages;
  destinations: DestinationMessages;
};

export function enMessages(): EnMessages {
  return {
    shell: shellMessages(),
    destinations: destinationMessages(),
  };
}
