import { expect, test } from 'vitest';
import { grantsSatisfy, isClientGrant, isClientPluginSlot, requiredGrantForSlot } from './types.ts';

test('the two client-plane slots of INT-081 are home-row and theme-pack, and nothing else', () => {
  expect(isClientPluginSlot('home-row')).toStrictEqual(true);
  expect(isClientPluginSlot('theme-pack')).toStrictEqual(true);
  expect(isClientPluginSlot('metadata-provider')).toStrictEqual(false);
  expect(isClientPluginSlot('scrobbler')).toStrictEqual(false);
  expect(isClientPluginSlot('')).toStrictEqual(false);
  expect(isClientPluginSlot('home-row ')).toStrictEqual(false);
});

test('the closed R1 grant set is home-row:read and theme-pack:apply', () => {
  expect(isClientGrant('home-row:read')).toStrictEqual(true);
  expect(isClientGrant('theme-pack:apply')).toStrictEqual(true);
  expect(isClientGrant('home-row:write')).toStrictEqual(false);
  expect(isClientGrant('library:read')).toStrictEqual(false);
  expect(isClientGrant('')).toStrictEqual(false);
  expect(isClientGrant('home-row')).toStrictEqual(false);
});

test('each slot requires exactly one grant of its own', () => {
  expect(requiredGrantForSlot('home-row')).toStrictEqual('home-row:read');
  expect(requiredGrantForSlot('theme-pack')).toStrictEqual('theme-pack:apply');
});

test('an empty requirement is satisfied by any grants, including none held', () => {
  expect(grantsSatisfy([], [])).toStrictEqual(true);
  expect(grantsSatisfy([], ['home-row:read'])).toStrictEqual(true);
});

test('a requirement is satisfied only when every required grant is held', () => {
  expect(grantsSatisfy(['home-row:read'], ['home-row:read'])).toStrictEqual(true);
  expect(grantsSatisfy(['home-row:read'], ['theme-pack:apply'])).toStrictEqual(false);
  expect(grantsSatisfy(['home-row:read'], [])).toStrictEqual(false);
  expect(grantsSatisfy(['home-row:read', 'theme-pack:apply'], ['theme-pack:apply', 'home-row:read'])).toStrictEqual(
    true,
  );
  expect(grantsSatisfy(['home-row:read', 'theme-pack:apply'], ['home-row:read'])).toStrictEqual(false);
});

test('duplicate grants neither help nor hurt', () => {
  expect(grantsSatisfy(['home-row:read', 'home-row:read'], ['home-row:read'])).toStrictEqual(true);
  expect(grantsSatisfy(['home-row:read'], ['home-row:read', 'home-row:read', 'theme-pack:apply'])).toStrictEqual(true);
});
