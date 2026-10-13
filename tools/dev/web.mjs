// Helper for tools/dev/web.ps1: serve the web bundle on localhost, and check
// it in a headless browser. Needs Node.js 22 and nothing else: the server is
// node:http, the browser is driven over the DevTools protocol with the
// built-in WebSocket, and the screenshots are read with node:zlib.
//
//   node tools/dev/web.mjs serve [--dir web/dist] [--port 8080]
//   node tools/dev/web.mjs check [--dir web/dist] [--out target/web]
//                                [--browser FILE] [--gl swiftshader|default]
//
// `check` passes only if the page loads without an error, the application
// starts, the canvas shows the application (including its placeholder
// banner), and a press of Space reaches it and changes the picture. It writes
// check-before.png, check-after.png, check-console.log and check.json to
// --out. The pictures show placeholder mode only; they are build output and
// are not committed (docs/LEGAL.md, "Screenshots").

import { spawn, spawnSync } from 'node:child_process';
import { createReadStream, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:http';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { setTimeout as sleep } from 'node:timers/promises';
import { inflateSync } from 'node:zlib';

/// The canvas of web/index.html, and the banner colour of the application
/// (apps/dragonlink/src/main.rs: `Color::srgb(1.0, 0.78, 0.25)`).
const CANVAS = '#dragonlink-canvas';
const BANNER_RGB = [255, 199, 64];

const VIEWPORT = { width: 1280, height: 900 };
const BOOT_TIMEOUT_MS = 120_000;
const RENDER_TIMEOUT_MS = 60_000;
const INPUT_TIMEOUT_MS = 30_000;
/// A rendered canvas has anti-aliased text, so thousands of colours; a blank
/// one has one. Anything under this is treated as blank.
const MIN_COLOURS = 64;
/// Pixels of the banner colour that must be visible (three lines of text).
const MIN_BANNER_PIXELS = 300;
/// Share of the canvas a spin must change. Five reels of three cells are far
/// more than this; a meter that only ticks over is far less.
const MIN_CHANGED_SHARE = 0.02;

/// Software rendering, so the check gives the same answer on a machine with
/// no GPU and in a session with no desktop. Verified with Edge 155 on
/// Windows 11; see docs/BUILDING.md.
const GL_FLAGS = {
    swiftshader: ['--use-angle=swiftshader', '--enable-unsafe-swiftshader'],
    default: [],
};

const MIME = {
    '.html': 'text/html; charset=utf-8',
    '.js': 'text/javascript; charset=utf-8',
    '.mjs': 'text/javascript; charset=utf-8',
    '.css': 'text/css; charset=utf-8',
    '.json': 'application/json',
    '.wasm': 'application/wasm',
    '.txt': 'text/plain; charset=utf-8',
};

function parseArgs(argv) {
    const [command, ...rest] = argv;
    const options = { dir: 'web/dist', out: 'target/web', port: '8080', gl: 'swiftshader', browser: process.env.DRAGONLINK_BROWSER ?? '' };
    for (let i = 0; i < rest.length; i += 2) {
        const name = rest[i].replace(/^--/, '');
        if (!rest[i].startsWith('--') || !(name in options) || rest[i + 1] === undefined) {
            throw new Error(`unknown or incomplete option ${rest[i]}`);
        }
        options[name] = rest[i + 1];
    }
    return { command, options };
}

/// Serve `dir` on 127.0.0.1 only. Nothing is cached, so a rebuilt bundle is
/// what the next reload shows.
function startServer(dir, port) {
    const root = path.resolve(dir);
    const server = createServer((request, response) => {
        const refuse = (status, text) => {
            response.writeHead(status, { 'Content-Type': 'text/plain; charset=utf-8' });
            response.end(text);
        };
        if (request.method !== 'GET' && request.method !== 'HEAD') return refuse(405, 'method not allowed');
        let name;
        try {
            name = decodeURIComponent(new URL(request.url, 'http://localhost').pathname);
        } catch {
            return refuse(400, 'bad request');
        }
        if (name.endsWith('/')) name += 'index.html';
        const file = path.resolve(root, `.${name}`);
        if (!file.startsWith(root + path.sep)) return refuse(403, 'forbidden');
        let stat;
        try {
            stat = statSync(file);
        } catch {
            return refuse(404, 'not found');
        }
        if (!stat.isFile()) return refuse(404, 'not found');
        response.writeHead(200, {
            'Content-Type': MIME[path.extname(file).toLowerCase()] ?? 'application/octet-stream',
            'Content-Length': stat.size,
            'Cache-Control': 'no-store',
        });
        if (request.method === 'HEAD') return response.end();
        createReadStream(file).pipe(response);
    });
    return new Promise((resolve, reject) => {
        server.once('error', reject);
        server.listen(port, '127.0.0.1', () => resolve(server));
    });
}

function requireBundle(dir) {
    for (const name of ['index.html', 'dragonlink.js', 'dragonlink_bg.wasm']) {
        if (!existsSync(path.join(dir, name))) {
            throw new Error(`${path.join(dir, name)} is missing: build the bundle first (tools/dev/web.ps1)`);
        }
    }
}

/// A Chromium-family browser: the one named, or the first usual one found.
function findBrowser(named) {
    if (named) {
        if (!existsSync(named)) throw new Error(`no browser at ${named}`);
        return named;
    }
    const env = process.env;
    const candidates = process.platform === 'win32'
        ? [
            [env['ProgramFiles(x86)'], 'Microsoft/Edge/Application/msedge.exe'],
            [env.ProgramFiles, 'Microsoft/Edge/Application/msedge.exe'],
            [env.ProgramFiles, 'Google/Chrome/Application/chrome.exe'],
            [env['ProgramFiles(x86)'], 'Google/Chrome/Application/chrome.exe'],
            [env.LOCALAPPDATA, 'Google/Chrome/Application/chrome.exe'],
        ].filter(([base]) => base).map(([base, rest]) => path.join(base, rest))
        : [
            '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
            '/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge',
            '/Applications/Chromium.app/Contents/MacOS/Chromium',
            ...['google-chrome', 'google-chrome-stable', 'chromium', 'chromium-browser', 'microsoft-edge']
                .flatMap((name) => (env.PATH ?? '').split(path.delimiter).map((folder) => path.join(folder, name))),
        ];
    const found = candidates.find((candidate) => existsSync(candidate));
    if (!found) {
        throw new Error('no Chromium-family browser found (Edge or Chrome); name one with --browser or DRAGONLINK_BROWSER');
    }
    return found;
}

/// One DevTools protocol connection.
class Cdp {
    constructor(socket) {
        this.socket = socket;
        this.nextId = 1;
        this.pending = new Map();
        this.listeners = new Map();
        socket.addEventListener('message', (message) => {
            const data = JSON.parse(message.data);
            if (data.id !== undefined) {
                const waiting = this.pending.get(data.id);
                this.pending.delete(data.id);
                if (!waiting) return;
                if (data.error) waiting.reject(new Error(`${waiting.method}: ${data.error.message}`));
                else waiting.resolve(data.result);
            } else {
                for (const listener of this.listeners.get(data.method) ?? []) listener(data.params);
            }
        });
        socket.addEventListener('close', () => {
            for (const waiting of this.pending.values()) waiting.reject(new Error(`${waiting.method}: the browser went away`));
            this.pending.clear();
        });
    }

    static connect(url) {
        return new Promise((resolve, reject) => {
            const socket = new WebSocket(url);
            socket.addEventListener('open', () => resolve(new Cdp(socket)), { once: true });
            socket.addEventListener('error', () => reject(new Error(`cannot connect to the browser at ${url}`)), { once: true });
        });
    }

    send(method, params = {}) {
        const id = this.nextId++;
        return new Promise((resolve, reject) => {
            this.pending.set(id, { resolve, reject, method });
            this.socket.send(JSON.stringify({ id, method, params }));
        });
    }

    on(event, listener) {
        this.listeners.set(event, [...(this.listeners.get(event) ?? []), listener]);
    }

    async evaluate(expression) {
        const { result, exceptionDetails } = await this.send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
        if (exceptionDetails) throw new Error(`evaluating in the page failed: ${exceptionDetails.text}`);
        return result.value;
    }
}

/// Start the browser with its own throwaway profile and wait for its
/// DevTools port, which it writes into that profile.
async function launchBrowser(executable, profile, glFlags) {
    const child = spawn(executable, [
        '--headless=new',
        '--remote-debugging-port=0',
        `--user-data-dir=${profile}`,
        `--window-size=${VIEWPORT.width},${VIEWPORT.height}`,
        '--no-first-run',
        '--no-default-browser-check',
        '--disable-extensions',
        '--disable-background-networking',
        '--hide-scrollbars',
        '--mute-audio',
        ...glFlags,
        'about:blank',
    ], { stdio: 'ignore' });
    let exited = false;
    child.once('exit', () => { exited = true; });
    child.once('error', () => { exited = true; });
    const portFile = path.join(profile, 'DevToolsActivePort');
    for (let waited = 0; waited < 30_000; waited += 100) {
        if (exited) throw new Error(`the browser exited before it could be driven (${executable})`);
        if (existsSync(portFile)) {
            const port = Number(readFileSync(portFile, 'utf8').split('\n')[0]);
            if (port > 0) return { child, port, exited: () => exited };
        }
        await sleep(100);
    }
    stopBrowser(child);
    throw new Error('the browser did not open a DevTools port within 30 s');
}

/// Stop the browser this script started, and only that one.
function stopBrowser(child) {
    if (child.exitCode !== null || child.pid === undefined) return;
    if (process.platform === 'win32') {
        spawnSync('taskkill', ['/PID', String(child.pid), '/T', '/F'], { stdio: 'ignore' });
    } else {
        child.kill('SIGKILL');
    }
}

/// Decode a PNG as the DevTools protocol writes them: 8 bits a channel, RGB or
/// RGBA, not interlaced.
function decodePng(bytes) {
    if (bytes.readUInt32BE(0) !== 0x89504e47) throw new Error('the screenshot is not a PNG');
    let width = 0;
    let height = 0;
    let channels = 0;
    const data = [];
    for (let at = 8; at < bytes.length;) {
        const length = bytes.readUInt32BE(at);
        const type = bytes.toString('latin1', at + 4, at + 8);
        const body = bytes.subarray(at + 8, at + 8 + length);
        if (type === 'IHDR') {
            width = body.readUInt32BE(0);
            height = body.readUInt32BE(4);
            const colourType = body[9];
            if (body[8] !== 8 || (colourType !== 2 && colourType !== 6) || body[12] !== 0) {
                throw new Error('the screenshot is a kind of PNG this check does not read');
            }
            channels = colourType === 6 ? 4 : 3;
        } else if (type === 'IDAT') {
            data.push(body);
        }
        at += 12 + length;
    }
    const raw = inflateSync(Buffer.concat(data));
    const stride = width * channels;
    const pixels = Buffer.alloc(stride * height);
    for (let y = 0; y < height; y++) {
        const filter = raw[y * (stride + 1)];
        const line = y * (stride + 1) + 1;
        const out = y * stride;
        for (let x = 0; x < stride; x++) {
            const left = x >= channels ? pixels[out + x - channels] : 0;
            const up = y > 0 ? pixels[out + x - stride] : 0;
            const upLeft = x >= channels && y > 0 ? pixels[out + x - stride - channels] : 0;
            let predicted = 0;
            if (filter === 1) predicted = left;
            else if (filter === 2) predicted = up;
            else if (filter === 3) predicted = (left + up) >> 1;
            else if (filter === 4) {
                const p = left + up - upLeft;
                const dl = Math.abs(p - left);
                const du = Math.abs(p - up);
                const dul = Math.abs(p - upLeft);
                predicted = dl <= du && dl <= dul ? left : du <= dul ? up : upLeft;
            }
            pixels[out + x] = (raw[line + x] + predicted) & 0xff;
        }
    }
    return { width, height, channels, pixels };
}

/// What the canvas part of a screenshot shows, in numbers.
function canvasStats(image, box) {
    const colours = new Map();
    let banner = 0;
    let total = 0;
    for (let y = box.top; y < box.bottom; y++) {
        for (let x = box.left; x < box.right; x++) {
            const at = (y * image.width + x) * image.channels;
            const [r, g, b] = [image.pixels[at], image.pixels[at + 1], image.pixels[at + 2]];
            const colour = (r << 16) | (g << 8) | b;
            colours.set(colour, (colours.get(colour) ?? 0) + 1);
            if (Math.abs(r - BANNER_RGB[0]) + Math.abs(g - BANNER_RGB[1]) + Math.abs(b - BANNER_RGB[2]) <= 24) banner++;
            total++;
        }
    }
    const [dominant, dominantCount] = [...colours].reduce((best, entry) => (entry[1] > best[1] ? entry : best), [0, 0]);
    return {
        width: box.right - box.left,
        height: box.bottom - box.top,
        colours: colours.size,
        dominant_colour: `#${dominant.toString(16).padStart(6, '0')}`,
        dominant_share: total ? Number((dominantCount / total).toFixed(4)) : 1,
        banner_pixels: banner,
    };
}

function changedShare(before, after, box) {
    if (before.width !== after.width || before.height !== after.height) return 1;
    let changed = 0;
    let total = 0;
    for (let y = box.top; y < box.bottom; y++) {
        for (let x = box.left; x < box.right; x++) {
            const a = (y * before.width + x) * before.channels;
            const b = (y * after.width + x) * after.channels;
            if (before.pixels[a] !== after.pixels[b] || before.pixels[a + 1] !== after.pixels[b + 1] || before.pixels[a + 2] !== after.pixels[b + 2]) changed++;
            total++;
        }
    }
    return total ? changed / total : 0;
}

const isRendered = (stats) => stats.colours >= MIN_COLOURS && stats.banner_pixels >= MIN_BANNER_PIXELS;

async function check(options) {
    const say = (label, text) => console.log(`web-check: ${label.padEnd(9)} ${text}`);
    const problems = [];
    const consoleLines = [];
    const started = Date.now();
    const note = (kind, text) => consoleLines.push(`${((Date.now() - started) / 1000).toFixed(3).padStart(8)}  ${kind.padEnd(9)} ${text}`);
    /// Anything the page reports as an error fails the check.
    const pageError = (kind, text) => {
        note(kind, text);
        problems.push(`${kind}: ${text.split('\n')[0]}`);
    };

    requireBundle(options.dir);
    if (!(options.gl in GL_FLAGS)) throw new Error(`--gl is swiftshader or default, not ${options.gl}`);
    mkdirSync(options.out, { recursive: true });
    const executable = findBrowser(options.browser);
    const server = await startServer(options.dir, 0);
    const url = `http://127.0.0.1:${server.address().port}/`;
    const profile = mkdtempSync(path.join(tmpdir(), 'dragonlink-web-check-'));
    const summary = { schema: 'dragonlink-web-check/1', passed: false, gl: options.gl, flags: GL_FLAGS[options.gl] };
    let browser;
    try {
        browser = await launchBrowser(executable, profile, GL_FLAGS[options.gl]);
        const targets = await (await fetch(`http://127.0.0.1:${browser.port}/json/list`)).json();
        const page = targets.find((target) => target.type === 'page');
        if (!page) throw new Error('the browser has no page to drive');
        const cdp = await Cdp.connect(page.webSocketDebuggerUrl);

        cdp.on('Runtime.consoleAPICalled', ({ type, args }) => {
            const text = args.map((arg) => arg.value ?? arg.description ?? arg.type).join(' ');
            if (type === 'error' || type === 'assert') pageError(`console.${type}`, text);
            else note(`console.${type}`, text);
        });
        cdp.on('Runtime.exceptionThrown', ({ exceptionDetails }) => {
            pageError('exception', exceptionDetails.exception?.description ?? exceptionDetails.text);
        });
        cdp.on('Log.entryAdded', ({ entry }) => {
            const text = `[${entry.source}] ${entry.text}${entry.url ? ` (${entry.url})` : ''}`;
            if (entry.level === 'error') pageError('log.error', text);
            else note(`log.${entry.level}`, text);
        });
        cdp.on('Network.loadingFailed', ({ errorText, canceled }) => {
            if (!canceled) pageError('network', errorText);
        });
        cdp.on('Network.responseReceived', ({ response }) => {
            if (response.status >= 400) pageError('network', `${response.status} ${response.url}`);
        });
        await cdp.send('Runtime.enable');
        await cdp.send('Log.enable');
        await cdp.send('Network.enable');
        await cdp.send('Page.enable');
        await cdp.send('Emulation.setDeviceMetricsOverride', { ...VIEWPORT, deviceScaleFactor: 1, mobile: false });
        await cdp.send('Emulation.setFocusEmulationEnabled', { enabled: true });

        const version = await cdp.send('Browser.getVersion');
        summary.browser = version.product;
        say('browser', `${version.product}, --headless=new ${GL_FLAGS[options.gl].join(' ')}`.trimEnd());

        const screenshot = async (name) => {
            const { data } = await cdp.send('Page.captureScreenshot', { format: 'png' });
            const bytes = Buffer.from(data, 'base64');
            writeFileSync(path.join(options.out, name), bytes);
            return decodePng(bytes);
        };
        const canvasBox = async (image) => {
            const rect = await cdp.evaluate(`(() => { const r = document.querySelector('${CANVAS}').getBoundingClientRect(); return { left: r.left, top: r.top, right: r.right, bottom: r.bottom }; })()`);
            return {
                left: Math.max(0, Math.ceil(rect.left)),
                top: Math.max(0, Math.ceil(rect.top)),
                right: Math.min(image.width, Math.floor(rect.right)),
                bottom: Math.min(image.height, Math.floor(rect.bottom)),
            };
        };

        // 1. The page loads and the application starts.
        await cdp.send('Page.navigate', { url });
        let boot = 'loading';
        const bootStarted = Date.now();
        while (Date.now() - bootStarted < BOOT_TIMEOUT_MS) {
            boot = await cdp.evaluate('document.documentElement.dataset.boot ?? "no page"').catch(() => 'loading');
            if (boot === 'running' || boot === 'failed') break;
            await sleep(250);
        }
        summary.boot = boot;
        summary.boot_seconds = Number(((Date.now() - bootStarted) / 1000).toFixed(1));
        if (boot === 'running') {
            say('boot', `the application started after ${summary.boot_seconds} s`);
        } else {
            const shown = await cdp.evaluate('document.getElementById("boot-status")?.textContent ?? ""').catch(() => '');
            problems.push(boot === 'failed'
                ? `the page says the application failed: ${shown.replace(/\s+/g, ' ').trim()}`
                : `the application did not start within ${BOOT_TIMEOUT_MS / 1000} s (state: ${boot})`);
        }
        summary.renderer = await cdp.evaluate(`(() => {
            const gl = document.createElement('canvas').getContext('webgl2');
            if (!gl) return 'no WebGL2';
            const info = gl.getExtension('WEBGL_debug_renderer_info');
            return info ? gl.getParameter(info.UNMASKED_RENDERER_WEBGL) : gl.getParameter(gl.RENDERER);
        })()`).catch((error) => `unknown (${error.message})`);
        say('renderer', summary.renderer);
        if (summary.renderer === 'no WebGL2') problems.push('the browser provides no WebGL2 with these flags');

        // 2. The canvas shows the application.
        let before = await screenshot('check-before.png');
        let box = await canvasBox(before);
        let stats = canvasStats(before, box);
        const renderStarted = Date.now();
        while (boot === 'running' && !isRendered(stats) && Date.now() - renderStarted < RENDER_TIMEOUT_MS) {
            await sleep(500);
            before = await screenshot('check-before.png');
            box = await canvasBox(before);
            stats = canvasStats(before, box);
        }
        if (boot === 'running' && isRendered(stats)) {
            // Let the first frames settle, so the picture compared against is still.
            await sleep(1000);
            before = await screenshot('check-before.png');
            stats = canvasStats(before, box);
        }
        summary.canvas = stats;
        say('canvas', `${stats.width}x${stats.height}, ${stats.colours} colours, ${(stats.dominant_share * 100).toFixed(1)}% ${stats.dominant_colour}, ${stats.banner_pixels} banner pixels`);
        if (stats.colours < MIN_COLOURS) {
            problems.push(`the canvas is blank: ${stats.colours} colours, ${(stats.dominant_share * 100).toFixed(1)}% of it ${stats.dominant_colour}`);
        } else if (stats.banner_pixels < MIN_BANNER_PIXELS) {
            problems.push(`the canvas does not show the placeholder banner (${stats.banner_pixels} pixels of its colour)`);
        }

        // 3. The keyboard reaches the application: Space spins.
        if (boot === 'running' && isRendered(stats)) {
            const focused = await cdp.evaluate('document.activeElement ? document.activeElement.id : ""');
            summary.focused = focused;
            if (`#${focused}` !== CANVAS) problems.push(`the canvas does not have keyboard focus after loading (focus is on "${focused}")`);
            const key = { key: ' ', code: 'Space', windowsVirtualKeyCode: 32, nativeVirtualKeyCode: 32 };
            await cdp.send('Input.dispatchKeyEvent', { type: 'rawKeyDown', ...key });
            await sleep(100);
            await cdp.send('Input.dispatchKeyEvent', { type: 'keyUp', ...key });
            let after = before;
            let share = 0;
            let previous = -1;
            const inputStarted = Date.now();
            // Wait until the picture has changed and then stopped changing.
            while (Date.now() - inputStarted < INPUT_TIMEOUT_MS) {
                await sleep(1000);
                after = await screenshot('check-after.png');
                share = changedShare(before, after, box);
                if (share >= MIN_CHANGED_SHARE && share === previous) break;
                previous = share;
            }
            summary.changed_share = Number(share.toFixed(4));
            summary.after = canvasStats(after, box);
            summary.scroll_y = await cdp.evaluate('window.scrollY');
            say('input', `Space changed ${(share * 100).toFixed(1)}% of the canvas`);
            if (share < MIN_CHANGED_SHARE) problems.push(`pressing Space changed only ${(share * 100).toFixed(2)}% of the canvas: the key did not start a spin`);
            if (summary.scroll_y !== 0) problems.push('pressing Space scrolled the page');
        }
        await sleep(250);
        await cdp.send('Browser.close').catch(() => {});
    } finally {
        if (browser) {
            for (let waited = 0; waited < 5000 && !browser.exited(); waited += 100) await sleep(100);
            stopBrowser(browser.child);
        }
        server.closeAllConnections();
        await new Promise((resolve) => server.close(resolve));
        rmSync(profile, { recursive: true, force: true, maxRetries: 20, retryDelay: 250 });
    }

    writeFileSync(path.join(options.out, 'check-console.log'), `${consoleLines.join('\n')}\n`);
    say('console', `${consoleLines.length} lines in ${path.join(options.out, 'check-console.log')}`);
    summary.passed = problems.length === 0;
    summary.problems = problems;
    writeFileSync(path.join(options.out, 'check.json'), `${JSON.stringify(summary, null, 2)}\n`);
    for (const problem of problems) say('FAIL', problem);
    say(summary.passed ? 'PASSED' : 'FAILED', `pictures and summary in ${options.out}`);
    return summary.passed ? 0 : 1;
}

async function serve(options) {
    requireBundle(options.dir);
    const server = await startServer(options.dir, Number(options.port));
    console.log(`web: serving ${options.dir} at http://127.0.0.1:${server.address().port}/  (Ctrl+C stops)`);
    await new Promise((resolve) => {
        process.once('SIGINT', resolve);
        process.once('SIGTERM', resolve);
    });
    server.closeAllConnections();
    await new Promise((resolve) => server.close(resolve));
    return 0;
}

try {
    const { command, options } = parseArgs(process.argv.slice(2));
    if (command === 'serve') process.exitCode = await serve(options);
    else if (command === 'check') process.exitCode = await check(options);
    else throw new Error('usage: node tools/dev/web.mjs serve|check [--dir DIR] [--port N] [--out DIR] [--browser FILE] [--gl swiftshader|default]');
} catch (error) {
    console.error(`web: ${error.message}`);
    process.exitCode = 2;
}
