<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { checkoutUrl, captureLicense, cachedUnlock, saveLicense, verifyLicense } from './lib/license';
  import { cueEvery, gameDetails, initialCue, nextCue, scoreSample, type Cue, type GameId, type MotionSample, type PlayerRound } from './lib/game';

  type Screen = 'home' | 'host' | 'controller' | 'privacy' | 'terms' | 'notFound';
  type Player = { id: string; name: string; calibrated: boolean; connected: boolean };
  type Wire = Record<string, unknown> & { type?: string };

  const demoStorageKey = 'demo:pairplay-motion';
  const samplePlayers: Player[] = [
    { id: 'sample-ada', name: 'Ada', calibrated: true, connected: true },
    { id: 'sample-lin', name: 'Lin', calibrated: true, connected: true },
  ];
  const screenFromPath = (path: string): Screen => path === '/' || path === '/demo' ? 'home' : path === '/privacy' ? 'privacy' : path === '/terms' ? 'terms' : 'notFound';
  let isDemo = location.pathname === '/demo' || new URLSearchParams(location.search).get('demo') === '1';
  let screen: Screen = isDemo ? 'host' : screenFromPath(location.pathname);
  let online = navigator.onLine;
  let busy = false;
  let error = '';
  let notice = '';
  let roomCode = new URLSearchParams(location.search).get('join')?.toUpperCase() || '';
  let playerName = '';
  let hostToken = '';
  let socket: WebSocket | null = null;
  let connection: 'idle' | 'connecting' | 'live' | 'lost' = 'idle';
  let players: Player[] = [];
  let qrCanvas: HTMLCanvasElement;
  let copied = false;
  let controllerId = '';
  let sensorMode: 'standby' | 'waiting' | 'calibrating' | 'motion' | 'touch' | 'denied' = 'standby';
  let controllerMessage = 'Waiting for the host.';
  let controllerCue: Cue | '' = '';
  let controllerScore = 0;
  let premium = false;
  let licenseNotice = '';
  let licenseInput = '';
  let selectedGame: GameId | null = null;
  let phase: 'lobby' | 'round' | 'results' = 'lobby';
  let cue: Cue = 'MOVE';
  let timeLeft = 0;
  let roundPlayers: Record<string, PlayerRound> = {};
  let tickTimer: number | undefined;
  let cueAt = 0;
  let roundStartedAt = 0;
  let lastBoardSent = 0;
  let baseBeta = 0;
  let baseGamma = 0;
  let liveBeta = 0;
  let liveGamma = 0;
  let liveShake = 0;
  let sampleAt = 0;
  let calibrationSamples: Array<[number, number]> = [];
  let orientationListening = false;

  const joinUrl = () => `${location.origin}/?join=${roomCode}`;
  const wsBase = () => `${location.protocol === 'https:' ? 'wss:' : 'ws:'}//${location.host}/ws`;
  const send = (message: Wire) => socket?.readyState === WebSocket.OPEN && socket.send(JSON.stringify(message));

  function pageTitle() {
    if (isDemo) return 'Demo — PairPlay Motion';
    if (screen === 'privacy') return 'Privacy — PairPlay Motion';
    if (screen === 'terms') return 'Terms — PairPlay Motion';
    if (screen === 'notFound') return 'Page not found — PairPlay Motion';
    if (screen === 'host') return 'Host a room — PairPlay Motion';
    if (screen === 'controller') return 'Phone controller — PairPlay Motion';
    return 'PairPlay Motion — phone motion games';
  }

  async function announceRoute() {
    document.title = pageTitle();
    await tick();
    document.querySelector<HTMLElement>('main h1')?.focus();
  }

  function loadDemoState() {
    isDemo = true;
    screen = 'host';
    roomCode = 'SAMPLE';
    hostToken = '';
    socket?.close(); socket = null;
    connection = 'live';
    players = samplePlayers.map((player) => ({ ...player }));
    selectedGame = 'freeze';
    phase = 'results';
    cue = 'FREEZE';
    timeLeft = 0;
    roundPlayers = {
      'sample-ada': { id: 'sample-ada', score: 34, hitCue: false, lastShakeAt: 0 },
      'sample-lin': { id: 'sample-lin', score: 28, hitCue: false, lastShakeAt: 0 },
    };
    sensorMode = 'standby';
    controllerCue = '';
    error = '';
    notice = 'Sample room loaded. Nothing is saved.';
    sessionStorage.setItem(demoStorageKey, 'loaded');
    document.title = pageTitle();
  }

  function startDemo() {
    history.pushState({}, '', '/demo');
    loadDemoState();
    void announceRoute();
  }

  function resetDemo() {
    loadDemoState();
  }

  function startForReal() {
    stopRound();
    socket?.close(); socket = null;
    sessionStorage.removeItem(demoStorageKey);
    isDemo = false;
    screen = 'home';
    roomCode = '';
    players = [];
    selectedGame = null;
    phase = 'lobby';
    roundPlayers = {};
    connection = 'idle';
    notice = '';
    error = '';
    history.pushState({}, '', '/');
    void announceRoute();
  }

  function navigate(next: Screen) {
    if (isDemo) { startForReal(); if (next === 'home') return; }
    stopRound();
    socket?.close();
    socket = null;
    screen = next;
    error = '';
    notice = '';
    const path = next === 'privacy' ? '/privacy' : next === 'terms' ? '/terms' : next === 'notFound' ? '/404' : '/';
    history.pushState({}, '', path);
    window.scrollTo({ top: 0, behavior: matchMedia('(prefers-reduced-motion: reduce)').matches ? 'auto' : 'smooth' });
    void announceRoute();
  }

  onMount(() => {
    if (isDemo) {
      loadDemoState();
    } else {
      captureLicense();
      premium = cachedUnlock();
      verifyLicense().then((result) => { premium = result.unlocked; licenseNotice = result.notice; });
      if (!sessionStorage.getItem('pairplay-view')) {
        fetch('/api/page-view', { method: 'POST' }).catch(() => undefined);
        sessionStorage.setItem('pairplay-view', '1');
      }
    }
    document.title = pageTitle();
    const onOnline = () => { online = true; };
    const onOffline = () => { online = false; };
    const onPop = () => {
      const nextDemo = location.pathname === '/demo' || new URLSearchParams(location.search).get('demo') === '1';
      if (nextDemo) loadDemoState();
      else { isDemo = false; screen = screenFromPath(location.pathname); void announceRoute(); }
    };
    addEventListener('online', onOnline); addEventListener('offline', onOffline); addEventListener('popstate', onPop);
    return () => {
      removeEventListener('online', onOnline); removeEventListener('offline', onOffline); removeEventListener('popstate', onPop);
      stopRound(); socket?.close(); removeMotionListeners();
    };
  });

  async function createRoom() {
    if (!online) { error = 'You are offline. Reconnect to create a room.'; return; }
    busy = true; error = '';
    try {
      const response = await fetch('/api/rooms', { method: 'POST', headers: { 'content-type': 'application/json' }, body: '{}' });
      if (!response.ok) throw new Error('Room service did not answer');
      const data = await response.json() as { code: string; host_token: string };
      roomCode = data.code; hostToken = data.host_token; screen = 'host'; void announceRoute();
      connectHost();
      setTimeout(async () => {
        const QRCode = (await import('qrcode')).default;
        await QRCode.toCanvas(qrCanvas, joinUrl(), { width: 216, margin: 1, color: { dark: '#171714', light: '#fffdf5' } });
      }, 0);
    } catch { error = 'Could not open a room. Check the connection and try again.'; }
    finally { busy = false; }
  }

  function connectHost() {
    if (isDemo) { connection = 'live'; return; }
    connection = 'connecting';
    socket?.close();
    const nextSocket = new WebSocket(`${wsBase()}?room=${encodeURIComponent(roomCode)}&role=host&token=${encodeURIComponent(hostToken)}`);
    socket = nextSocket;
    nextSocket.onopen = () => { if (socket === nextSocket) connection = 'live'; };
    nextSocket.onclose = () => { if (socket === nextSocket) { connection = 'lost'; if (phase === 'round') stopRound(); } };
    nextSocket.onerror = () => { if (socket === nextSocket) error = 'The room connection failed. Reconnect without creating a new code.'; };
    nextSocket.onmessage = (event) => handleHostMessage(JSON.parse(event.data));
  }

  function handleHostMessage(message: Wire) {
    if (message.type === 'roster') {
      const roster = message.players as Array<{ id: string; name: string }>;
      players = roster.map((incoming) => {
        const existing = players.find((player) => player.id === incoming.id);
        return { ...incoming, calibrated: existing?.calibrated || false, connected: true };
      });
    }
    if (message.type === 'calibrated') {
      players = players.map((player) => player.id === message.playerId ? { ...player, calibrated: true } : player);
      notice = `${String(message.name || 'A player')} is calibrated.`;
    }
    if (message.type === 'motion' && phase === 'round') {
      const id = String(message.playerId);
      const target = roundPlayers[id];
      if (target) {
        target.prior = target.sample;
        target.sample = { x: Number(message.x) || 0, y: Number(message.y) || 0, shake: Number(message.shake) || 0, at: Number(message.at) || Date.now() };
      }
    }
  }

  async function copyCode() {
    await navigator.clipboard.writeText(`${roomCode} — ${joinUrl()}`);
    copied = true; setTimeout(() => copied = false, 1800);
  }

  function askCalibration() {
    players = players.map((player) => ({ ...player, calibrated: false }));
    send({ type: 'calibrate' });
    notice = 'Calibration sent. Each player must approve motion or choose touch controls.';
  }

  function joinRoom() {
    roomCode = roomCode.replace(/[^A-Z0-9]/g, '').slice(0, 6);
    playerName = playerName.trim().slice(0, 24);
    if (roomCode.length !== 6) { error = 'Enter the six-character room code.'; return; }
    if (playerName.length < 1) { error = 'Enter a name for the scoreboard.'; return; }
    if (!online) { error = 'You are offline. Reconnect before joining.'; return; }
    error = ''; screen = 'controller'; connection = 'connecting'; void announceRoute();
    const nextSocket = new WebSocket(`${wsBase()}?room=${encodeURIComponent(roomCode)}&role=controller&name=${encodeURIComponent(playerName)}`);
    socket = nextSocket;
    nextSocket.onopen = () => { if (socket === nextSocket) { connection = 'live'; controllerMessage = 'Connected. Look at the host screen.'; } };
    nextSocket.onclose = (event) => {
      if (socket !== nextSocket) return;
      connection = 'lost';
      controllerMessage = event.code === 4004 ? 'Room not found. Check the code with the host.' : event.code === 4003 ? 'This room already has four players.' : 'Connection lost. Rejoin when Wi-Fi returns.';
    };
    // Browsers intentionally hide the HTTP status of a failed WebSocket
    // handshake. The server upgrades then closes with an application code so
    // the recovery message below is reliable for full and missing rooms.
    nextSocket.onerror = () => undefined;
    nextSocket.onmessage = (event) => handleControllerMessage(JSON.parse(event.data));
  }

  function handleControllerMessage(message: Wire) {
    if (message.type === 'welcome') controllerId = String(message.player_id);
    if (message.type === 'calibrate') { sensorMode = 'waiting'; controllerMessage = 'Ready to calibrate. Keep the phone upright and still.'; }
    if (message.type === 'game_start') {
      controllerCue = message.cue as Cue; controllerScore = 0;
      controllerMessage = `Round started: ${gameDetails[message.game as GameId].name}`;
    }
    if (message.type === 'cue') { controllerCue = message.cue as Cue; controllerMessage = cueInstruction(controllerCue); }
    if (message.type === 'scoreboard') {
      const scores = message.scores as Record<string, number>;
      controllerScore = scores?.[controllerId] || 0;
    }
    if (message.type === 'round_end') { controllerCue = ''; controllerMessage = `Round complete. You scored ${controllerScore}.`; }
  }

  function cueInstruction(value: Cue) {
    if (value === 'FREEZE') return 'Freeze. Hold perfectly still.';
    if (value === 'MOVE') return 'Move. Get ready to stop.';
    if (value === 'GO') return 'Go. Shake the phone.';
    if (value === 'REST') return 'Rest. Hold still.';
    return `Tilt ${value.toLowerCase()}.`;
  }

  async function enableMotion() {
    sensorMode = 'calibrating'; error = '';
    try {
      const Orientation = DeviceOrientationEvent as unknown as { requestPermission?: () => Promise<string> };
      const Motion = DeviceMotionEvent as unknown as { requestPermission?: () => Promise<string> };
      if (Orientation.requestPermission && await Orientation.requestPermission() !== 'granted') throw new Error('denied');
      if (Motion.requestPermission && await Motion.requestPermission() !== 'granted') throw new Error('denied');
      addMotionListeners(); calibrationSamples = [];
      controllerMessage = 'Calibrating… keep the phone upright and still for two seconds.';
      window.setTimeout(() => {
        if (calibrationSamples.length < 3) {
          removeMotionListeners(); sensorMode = 'denied';
          error = 'No motion readings arrived. This browser can still play with touch controls.';
          return;
        }
        baseBeta = calibrationSamples.reduce((sum, value) => sum + value[0], 0) / calibrationSamples.length;
        baseGamma = calibrationSamples.reduce((sum, value) => sum + value[1], 0) / calibrationSamples.length;
        sensorMode = 'motion'; controllerMessage = 'Motion ready. Keep this screen awake.';
        send({ type: 'calibrated', mode: 'motion', name: playerName });
      }, 2000);
    } catch {
      sensorMode = 'denied';
      error = 'Motion access was not granted. You can still play with touch controls.';
    }
  }

  function addMotionListeners() {
    if (orientationListening) return;
    orientationListening = true;
    addEventListener('deviceorientation', onOrientation);
    addEventListener('devicemotion', onDeviceMotion);
  }
  function removeMotionListeners() {
    removeEventListener('deviceorientation', onOrientation);
    removeEventListener('devicemotion', onDeviceMotion); orientationListening = false;
  }
  function onOrientation(event: DeviceOrientationEvent) {
    liveBeta = event.beta || 0; liveGamma = event.gamma || 0;
    if (sensorMode === 'calibrating') calibrationSamples.push([liveBeta, liveGamma]);
    transmitMotion();
  }
  function onDeviceMotion(event: DeviceMotionEvent) {
    const acceleration = event.accelerationIncludingGravity;
    if (acceleration) liveShake = Math.abs(Math.hypot(acceleration.x || 0, acceleration.y || 0, acceleration.z || 0) - 9.81);
    transmitMotion();
  }
  function transmitMotion(force = false) {
    const now = Date.now();
    if (!force && now - sampleAt < 80) return;
    sampleAt = now;
    send({ type: 'motion', x: liveGamma - baseGamma, y: liveBeta - baseBeta, shake: liveShake, at: now });
    liveShake *= 0.5;
  }

  function useTouch() {
    removeMotionListeners(); sensorMode = 'touch'; error = '';
    controllerMessage = 'Touch controls ready. Arrows tilt; the center button shakes.';
    send({ type: 'calibrated', mode: 'touch', name: playerName });
  }

  function touchMotion(x: number, y: number, shake = 0) {
    if (sensorMode !== 'touch') return;
    liveGamma = x; liveBeta = y; liveShake = shake; transmitMotion(true);
  }
  function pulseMotion(x: number, y: number, shake = 0) {
    touchMotion(x, y, shake);
    window.setTimeout(() => touchMotion(0, 0), 180);
  }

  function keyboardMotion(event: KeyboardEvent) {
    if (screen !== 'controller' || sensorMode !== 'touch') return;
    const motion: Record<string, [number, number, number]> = { ArrowLeft: [-24, 0, 0], ArrowRight: [24, 0, 0], ArrowUp: [0, -24, 0], ArrowDown: [0, 24, 0], ' ': [0, 0, 10] };
    if (motion[event.key]) { event.preventDefault(); pulseMotion(...motion[event.key]); }
  }

  function startGame(game: GameId) {
    if (gameDetails[game].paid && !premium) return;
    selectedGame = game; phase = 'round'; cue = initialCue(game); timeLeft = 40; cueAt = Date.now(); roundStartedAt = cueAt;
    roundPlayers = Object.fromEntries(players.map((player) => [player.id, { id: player.id, score: 0, hitCue: false, lastShakeAt: 0 }]));
    send({ type: 'game_start', game, cue, duration: 40 });
    send({ type: 'cue', cue });
    tickTimer = window.setInterval(roundTick, 100);
  }

  function roundTick() {
    if (!selectedGame) return;
    const now = Date.now();
    timeLeft = Math.max(0, Math.ceil((40000 - (now - roundStartedAt)) / 1000));
    if (now - cueAt >= cueEvery(selectedGame)) {
      cue = nextCue(selectedGame, cue); cueAt = now;
      Object.values(roundPlayers).forEach((player) => { player.hitCue = false; });
      send({ type: 'cue', cue });
    }
    for (const [id, player] of Object.entries(roundPlayers)) {
      player.score += scoreSample(selectedGame, cue, player, now);
      if (player.sample) player.prior = { ...player.sample };
    }
    roundPlayers = { ...roundPlayers };
    if (now - lastBoardSent > 500) { broadcastScores(); lastBoardSent = now; }
    if (timeLeft <= 0) finishRound();
  }

  function scores() {
    return Object.fromEntries(Object.entries(roundPlayers).map(([id, player]) => [id, player.score]));
  }
  function broadcastScores() { send({ type: 'scoreboard', scores: scores() }); }
  function finishRound() {
    if (tickTimer) clearInterval(tickTimer); tickTimer = undefined;
    broadcastScores(); send({ type: 'round_end', scores: scores() }); phase = 'results';
  }
  function stopRound() { if (tickTimer) clearInterval(tickTimer); tickTimer = undefined; }
  function backToGames() { stopRound(); phase = 'lobby'; selectedGame = null; cue = 'MOVE'; }

  async function restoreLicense() {
    if (!licenseInput.trim()) { licenseNotice = 'Paste the license token from your receipt.'; return; }
    saveLicense(licenseInput); licenseNotice = 'Checking license…';
    const result = await verifyLicense(true); premium = result.unlocked; licenseNotice = result.unlocked ? 'Full edition unlocked on this device.' : result.notice;
    if (result.unlocked) licenseInput = '';
  }

  $: canCalibrate = players.length >= 2 && players.length <= 4;
  $: canPlay = canCalibrate && players.every((player) => player.calibrated) && connection === 'live';
  $: sortedResults = players.map((player) => ({ ...player, score: roundPlayers[player.id]?.score || 0 })).sort((a, b) => b.score - a.score);
</script>

<svelte:window onkeydown={keyboardMotion} />

<header class="masthead">
  <a class="brand" href="/" onclick={(event) => { event.preventDefault(); navigate('home'); }}>
    <span class="edition">Motion room games</span>
    <span class="brand-name">PairPlay Motion</span>
  </a>
  <nav aria-label="Primary navigation">
    <a href="/demo" onclick={(event) => { event.preventDefault(); startDemo(); }}>Demo</a>
    <a href="/privacy" onclick={(event) => { event.preventDefault(); navigate('privacy'); }}>Privacy</a>
    <a href="/terms" onclick={(event) => { event.preventDefault(); navigate('terms'); }}>Terms</a>
  </nav>
</header>

{#if !online}
  <div class="offline" role="status"><strong>OFFLINE</strong> Rooms need a connection. Reconnect to host or join.</div>
{/if}
{#if isDemo}
  <aside class="demo-banner" aria-label="Sample game mode"><strong>Demo — sample data, nothing is saved</strong><span><button onclick={resetDemo}>Reset demo</button><button onclick={startForReal}>Start for real</button></span></aside>
{/if}

<main id="main">
  <div class="live-region" aria-live="polite">{notice || licenseNotice}</div>
  {#if error}<p class="error page-error" role="alert">{error}</p>{/if}

  {#if screen === 'home'}
    <section class="hero" aria-labelledby="lead-title">
      <div class="hero-copy">
        <p class="slug">Room games for spare phones</p>
        <h1 id="lead-title" tabindex="-1">Turn phones into motion controllers</h1>
        <p class="dek">For friends and families who want a room game with spare phones.</p>
        <div class="actions">
          <button class="primary" onclick={startDemo}>Try it with sample data</button>
          <button onclick={createRoom} disabled={busy}>{busy ? 'Opening room…' : 'Host a game'}</button>
          <a class="text-link" href="#join">Join with a code</a>
        </div>
        <p class="action-note">The sample opens a finished two-player round. Hosting creates a new room code.</p>
        <ul class="facts" aria-label="Key facts"><li>No app or account is needed.</li><li>Sensor samples leave when the room ends.</li><li>Dead Still is free. Three games cost US $8 once.</li></ul>
      </div>
      <figure class="hero-art">
        <picture>
          <source media="(max-width: 700px)" srcset="/assets/hero-broadsheet-720.webp" />
          <img src="/assets/hero-broadsheet-1200.webp" width="1200" height="800" alt="Four hands hold plain phones around a table with red motion arcs." fetchpriority="high" />
        </picture>
        <figcaption>Original illustration of phones used as controllers.</figcaption>
      </figure>
    </section>

    <section class="how" aria-labelledby="how-title">
      <p class="section-no">How it works</p>
      <h2 id="how-title">Start a room</h2>
      <ol class="steps">
        <li><strong>Host a game.</strong><span>Put this browser on the shared screen.</span></li>
        <li><strong>Join with a code.</strong><span>Players use the six characters from the host.</span></li>
        <li><strong>Calibrate each phone.</strong><span>Allow motion, or use touch controls instead.</span></li>
      </ol>
    </section>

    <section id="join" class="join-desk" aria-labelledby="join-title">
      <div>
        <p class="section-no">Join a room</p>
        <h2 id="join-title">Enter a room code</h2>
        <p>Use the code from the host screen and choose a scoreboard name.</p>
      </div>
      <form onsubmit={(event) => { event.preventDefault(); joinRoom(); }}>
        <label for="room-code">Room code</label>
        <input id="room-code" bind:value={roomCode} maxlength="6" autocomplete="off" autocapitalize="characters" inputmode="text" required aria-describedby="code-help" />
        <small id="code-help">Six letters or numbers shown by the host.</small>
        <label for="player-name">Scoreboard name</label>
        <input id="player-name" bind:value={playerName} maxlength="24" autocomplete="nickname" required />
        <button class="primary" type="submit">Join room</button>
      </form>
    </section>

    <section class="games-preview" aria-labelledby="games-title">
      <p class="section-no">Games</p>
      <h2 id="games-title">Games included</h2>
      <div class="game-grid">
        {#each Object.entries(gameDetails) as [id, game], index}
          <article>
            <span class="game-number">{String(index + 1).padStart(2, '0')}</span>
            <p class="slug">{game.kicker}</p>
            <h3>{game.name}</h3>
            <p>{game.description}</p>
            <span class="access-mark">{game.paid ? 'Full edition' : 'Free to play'}</span>
          </article>
        {/each}
      </div>
    </section>

    <section class="edition-offer" aria-labelledby="edition-title">
      <div><p class="section-no">Full edition</p><h2 id="edition-title">Three games for US $8 once</h2><p>Dead Still stays free. A one-time license adds News Desk and Ink Runner. There is no subscription.</p></div>
      <div class="license-actions">
        <a class="primary button-link" href={checkoutUrl}>Buy the full edition</a>
        <label for="license">Paste a license token</label>
        <div class="inline-form"><input id="license" bind:value={licenseInput} autocomplete="off" /><button onclick={restoreLicense}>Restore license</button></div>
        {#if premium}<p class="success" role="status">Full edition is unlocked.</p>{/if}
        {#if licenseNotice}<p class="notice" role="status">{licenseNotice}</p>{/if}
      </div>
    </section>

  {:else if screen === 'host'}
    <section class="room-head" aria-labelledby="room-title">
      <div><p class="slug"><span class:live={connection === 'live'}>● {isDemo ? 'SAMPLE ROOM' : connection === 'live' ? 'LIVE ROOM' : connection.toUpperCase()}</span></p><h1 id="room-title" tabindex="-1">Room <span class="room-code">{roomCode}</span></h1><p>{isDemo ? 'This sample has two calibrated players and a finished round.' : 'Keep this tab open while players join with the room code.'}</p></div>
      {#if !isDemo}<div class="qr-wrap">
        <div role="img" aria-label={`QR code to join room ${roomCode}`}><canvas bind:this={qrCanvas} width="216" height="216" aria-hidden="true"></canvas></div>
        <button class="quiet" onclick={copyCode}>{copied ? 'Copied' : 'Copy invite'}</button>
      </div>{/if}
    </section>

    {#if connection === 'lost'}<div class="state-box"><strong>Connection lost.</strong><p>Your room may still be available. Reconnect the host to continue.</p><button onclick={connectHost}>Reconnect room</button></div>{/if}

    {#if phase === 'lobby'}
      <section class="lobby" aria-labelledby="players-title">
        <div class="section-heading"><div><p class="section-no">Players / {players.length} of 4</p><h2 id="players-title">Players in this room</h2></div>{#if canCalibrate}<button onclick={askCalibration}>Calibrate phones</button>{/if}</div>
        {#if players.length === 0}
          <div class="empty-state"><span class="empty-mark">＋</span><h3>Waiting for players</h3><p>Scan the code or enter <strong>{roomCode}</strong> on each phone.</p></div>
        {:else}
          <ol class="player-list">
            {#each players as player, index}
              <li><span class="player-no">{index + 1}</span><strong>{player.name}</strong><span class:ready={player.calibrated}>{player.calibrated ? 'Calibrated' : 'Needs calibration'}</span></li>
            {/each}
          </ol>
          {#if players.length < 2}<p class="notice">One more player is needed. Rooms allow two to four players.</p>{/if}
        {/if}
        {#if notice}<p class="notice" role="status">{notice}</p>{/if}
      </section>

      <section class="choose-game" aria-labelledby="choose-title">
        <p class="section-no">Choose a game</p><h2 id="choose-title">Start a round</h2>
        <div class="game-grid playable">
          {#each Object.entries(gameDetails) as [id, game], index}
            <article class:locked={game.paid && !premium}>
              <span class="game-number">{String(index + 1).padStart(2, '0')}</span><p class="slug">{game.kicker}</p><h3>{game.name}</h3><p>{game.description}</p>
              {#if game.paid && !premium}<a class="button-link" href={checkoutUrl}>Buy for US $8 once</a>{:else}<button class="primary" disabled={!canPlay} onclick={() => startGame(id as GameId)}>Play {game.name}</button>{/if}
            </article>
          {/each}
        </div>
        {#if !canPlay}<p class="notice">Start when two to four connected phones are calibrated.</p>{/if}
      </section>
    {:else}
      <section class="round" aria-labelledby="round-title">
        <div class="round-top"><p class="slug">{selectedGame ? gameDetails[selectedGame].name : ''} / {phase === 'round' ? 'LIVE' : 'RESULTS'}</p><span class="timer" aria-label={`${timeLeft} seconds remaining`}>{phase === 'round' ? timeLeft : 'END'}</span></div>
        <h2 id="round-title" class="cue">{phase === 'round' ? cue : 'Round results'}</h2>
        <p class="cue-help">{phase === 'round' ? cueInstruction(cue) : 'Scores from this round.'}</p>
        <ol class="scoreboard">
          {#each sortedResults as player, index}
            <li><span>{index + 1}</span><strong>{player.name}</strong><b>{player.score}</b></li>
          {/each}
        </ol>
        <div class="actions">{#if phase === 'round'}<button onclick={finishRound}>End round now</button>{:else}<button class="primary" onclick={() => selectedGame && startGame(selectedGame)}>Play again</button><button onclick={backToGames}>Choose another game</button>{/if}</div>
      </section>
    {/if}

  {:else if screen === 'controller'}
    <section class="controller" aria-labelledby="controller-title">
      <p class="slug"><span class:live={connection === 'live'}>● {connection === 'live' ? `ROOM ${roomCode} LIVE` : connection.toUpperCase()}</span></p>
      <h1 id="controller-title" tabindex="-1">{playerName || 'Phone controller'}</h1>
      <p class="controller-message" aria-live="polite">{controllerMessage}</p>
      {#if controllerCue}<div class="phone-cue" aria-label={`Current cue: ${controllerCue}`}>{controllerCue}</div><p class="phone-score">Score <strong>{controllerScore}</strong></p>{/if}

      {#if sensorMode === 'standby'}
        <div class="empty-state"><span class="empty-mark">◎</span><h2>Connected to the room</h2><p>The host will ask every phone to calibrate.</p></div>
      {:else if sensorMode === 'waiting' || sensorMode === 'denied'}
        <div class="permission-sheet">
          <p class="section-no">Motion permission</p><h2>Use this phone’s motion?</h2>
          <p>PairPlay reads tilt and acceleration while this controller page is open. Samples go only to this live room and are not saved.</p>
          <button class="primary" onclick={enableMotion}>Allow motion and calibrate</button>
          <button onclick={useTouch}>Use touch controls instead</button>
        </div>
      {:else if sensorMode === 'calibrating'}
        <div class="calibration" role="status"><div class="cross"><span></span></div><h2>Hold still</h2><p>Keep the phone upright until calibration completes.</p></div>
      {:else}
        <div class="motion-ready">
          <div class="cross" style={`--mx:${Math.max(-34, Math.min(34, liveGamma - baseGamma))}px;--my:${Math.max(-34, Math.min(34, liveBeta - baseBeta))}px`}><span></span></div>
          <p><strong>{sensorMode === 'motion' ? 'Motion calibrated' : 'Touch controls active'}</strong><br />Keep this screen open and awake.</p>
        </div>
      {/if}

      {#if sensorMode === 'touch'}
        <div class="touch-pad" aria-label="Touch motion controls">
          <button class="up" aria-label="Tilt up" onclick={() => pulseMotion(0, -24)}>↑</button>
          <button class="left" aria-label="Tilt left" onclick={() => pulseMotion(-24, 0)}>←</button>
          <button class="shake" aria-label="Shake" onclick={() => pulseMotion(0, 0, 10)}>SHAKE</button>
          <button class="right" aria-label="Tilt right" onclick={() => pulseMotion(24, 0)}>→</button>
          <button class="down" aria-label="Tilt down" onclick={() => pulseMotion(0, 24)}>↓</button>
        </div>
        <p class="key-help">Keyboard: arrow keys tilt. Space shakes.</p>
      {/if}
      {#if connection === 'lost'}<button class="primary" onclick={() => { screen = 'home'; joinRoom(); }}>Rejoin room</button>{/if}
    </section>

  {:else if screen === 'privacy'}
    <article class="legal"><p class="section-no">Policy / Effective 6 September 2026</p><h1 tabindex="-1">Privacy</h1><p class="dek">Your phone controls the game.</p><h2>What moves through a room</h2><p>Your scoreboard name and short motion samples go to the host during a live room. Rooms and motion are not written to the durable SQLite database.</p><h2>What is stored</h2><p>A page view changes only the daily aggregate count. A license token and its latest check stay only in your browser.</p><h2>Your choices</h2><p>You can deny motion and use touch controls. Close the controller tab to stop sensor reading. Clear site storage to remove a license. We do not run advertising trackers.</p><h2>Contact</h2><p>Send privacy questions to <a href="mailto:privacy@sociobot.in">privacy@sociobot.in</a>.</p></article>
  {:else if screen === 'terms'}
    <article class="legal"><p class="section-no">Terms / Effective 6 September 2026</p><h1 tabindex="-1">Terms</h1><p class="dek">PairPlay Motion is for ordinary personal use with people in the same room.</p><h2>Play safely</h2><p>Keep a firm grip, use a clear space, and do not throw or strike with a phone. An adult should supervise children. Sensor support varies by phone and browser. Touch controls are the fallback.</p><h2>Full edition</h2><p>The full edition costs US $8 once. It adds News Desk and Ink Runner. Sociobot/Dodo is the merchant of record for checkout and refunds. A refunded or revoked license stops unlocking paid games. Dead Still and accessibility features stay free.</p><h2>Using the service</h2><p>Do not disrupt the relay, inspect other rooms, or use the service unlawfully. Rooms are temporary and availability is not guaranteed. To the extent allowed by law, liability is limited to the amount paid.</p><h2>Contact</h2><p>Send product questions to <a href="mailto:support@sociobot.in">support@sociobot.in</a>.</p></article>
  {:else}
    <article class="legal not-found"><p class="section-no">404</p><h1 tabindex="-1">Page not found</h1><p class="dek">This address does not point to a PairPlay page.</p><button class="primary" onclick={() => navigate('home')}>Go to the home page</button></article>
  {/if}
</main>

<footer><p>PairPlay Motion — motion games for a shared screen.</p><p>Original generated illustration. <a href="/privacy" onclick={(event) => { event.preventDefault(); navigate('privacy'); }}>Privacy</a> · <a href="/terms" onclick={(event) => { event.preventDefault(); navigate('terms'); }}>Terms</a> · Built by Param Factory · Build v1.0</p></footer>
