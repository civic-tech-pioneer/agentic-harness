// The settings page of the checked provider: the judge's thresholds and origin (TECHNICAL 10 "Settings").
const $ = (id) => document.getElementById(id);

async function read() {
  const s = await watane.call('settings.get');
  $('origin').textContent = s.origin;
  $('unsafe-at').value = s['unsafe-at'];
  $('safe-at').value = s['safe-at'];
}

function say(text, bad) {
  $('status').textContent = text;
  $('status').className = bad ? 'error' : '';
}

$('form').addEventListener('submit', async (event) => {
  event.preventDefault();
  try {
    const r = await watane.call('settings.set', {
      'unsafe-at': Number($('unsafe-at').value),
      'safe-at': Number($('safe-at').value),
    });
    if (r && r.err) {
      say(r.err.refused || r.err, true);
    } else {
      say('Saved.', false);
    }
  } catch (e) {
    say(e.message || String(e), true);
  }
});

watane.on((message) => {
  if (message.type === 'changed') read().catch((e) => say(e.message || String(e), true));
});
