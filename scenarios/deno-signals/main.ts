// https://github.com/moonrepo/moon/issues/2027

import {delay} from 'jsr:@std/async';

Deno.addSignalListener('SIGINT', () => {
  console.log('I was interrupted');
});

Deno.addSignalListener('SIGTERM', () => {
  console.log('I was terminated');
});

if (import.meta.main) {
  console.log("I'm starting now");

  while (true) {
    await delay(2000);
    console.log(Date.now());
  }
}
