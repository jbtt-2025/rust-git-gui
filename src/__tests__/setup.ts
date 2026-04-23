/**
 * Test setup: mock Tauri APIs and browser APIs missing from jsdom.
 */
import { vi } from 'vitest';

// ── Polyfill browser APIs missing from jsdom ──

// window.matchMedia
Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: vi.fn().mockImplementation((query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  })),
});

// ResizeObserver
class ResizeObserverMock {
  observe = vi.fn();
  unobserve = vi.fn();
  disconnect = vi.fn();
}
Object.defineProperty(window, 'ResizeObserver', {
  writable: true,
  value: ResizeObserverMock,
});

// HTMLCanvasElement.getContext
const noopFn = vi.fn();
HTMLCanvasElement.prototype.getContext = vi.fn().mockReturnValue({
  fillRect: noopFn,
  clearRect: noopFn,
  beginPath: noopFn,
  moveTo: noopFn,
  lineTo: noopFn,
  stroke: noopFn,
  arc: noopFn,
  fill: noopFn,
  save: noopFn,
  restore: noopFn,
  scale: noopFn,
  translate: noopFn,
  fillText: noopFn,
  strokeText: noopFn,
  measureText: vi.fn().mockReturnValue({ width: 0 }),
  setTransform: noopFn,
  createLinearGradient: vi.fn().mockReturnValue({ addColorStop: noopFn }),
  quadraticCurveTo: noopFn,
  bezierCurveTo: noopFn,
  closePath: noopFn,
  rect: noopFn,
  clip: noopFn,
  roundRect: noopFn,
  setLineDash: noopFn,
  getLineDash: vi.fn().mockReturnValue([]),
  canvas: { width: 800, height: 600 },
  lineWidth: 1,
  strokeStyle: '',
  fillStyle: '',
  font: '',
  textAlign: '',
  textBaseline: '',
  globalAlpha: 1,
  globalCompositeOperation: 'source-over',
  lineCap: 'butt',
  lineJoin: 'miter',
}) as unknown as typeof HTMLCanvasElement.prototype.getContext;

// Mock @tauri-apps/api/core
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockRejectedValue(new Error('invoke not mocked for this command')),
}));

// Mock @tauri-apps/api/event
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
  emit: vi.fn().mockResolvedValue(undefined),
}));

// Mock @tauri-apps/plugin-shell
vi.mock('@tauri-apps/plugin-shell', () => ({
  Command: {
    create: vi.fn().mockReturnValue({
      spawn: vi.fn().mockResolvedValue({
        write: vi.fn(),
        kill: vi.fn(),
      }),
      on: vi.fn(),
    }),
  },
}));
