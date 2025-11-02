<script>
  import { onMount } from 'svelte';
  import { defaultCode } from '../defaultCode';

  let editorContainer;
  let outputContainer;
  let resizerElem;
  let sandboxIframe;
  let editor;
  let isReady = false;
  let editorReady = false;
  let statusText = 'Initializing...';
  let isDragging = false;

  // URL hashing helpers (linkhash style)
  function decodeFromUrl(hash) {
    try {
      return decodeURIComponent(escape(window.atob(hash)));
    } catch (e) {
      console.error('Failed to decode from URL:', e);
      return defaultCode;
    }
  }

  function encodeForUrl(code) {
    return window.btoa(unescape(encodeURIComponent(code)));
  }

  function updateUrl(code) {
    const encoded = encodeForUrl(code);
    const url = new URL(window.location);
    url.searchParams.set('h', encoded);
    window.history.replaceState({}, '', url);
  }

  function debounce(func, wait) {
    let timeout;
    return function executedFunction(...args) {
      const later = () => {
        clearTimeout(timeout);
        func(...args);
      };
      clearTimeout(timeout);
      timeout = setTimeout(later, wait);
    };
  }

  const debouncedUpdateUrl = debounce(updateUrl, 600);

  // Auto-execute code (linkhash style - 600ms debounce)
  function executeCode(code) {
    if (!isReady || !sandboxIframe) {
      return;
    }

    // Send code to sandboxed iframe for execution
    sandboxIframe.contentWindow.postMessage({
      type: 'execute',
      code: code
    }, '*');
  }

  const debouncedExecute = debounce(executeCode, 600);

  // Get initial code from URL or use default
  function getInitialCode() {
    const urlParams = new URLSearchParams(window.location.search);
    const hash = urlParams.get('h');
    if (hash) {
      return decodeFromUrl(hash);
    }
    return defaultCode;
  }

  const initialCode = getInitialCode();

  onMount(async () => {
    // Load Monaco Editor
    await loadMonaco();
    editorReady = true;

    // Execute initial code if sandbox is already ready
    if (isReady) {
      executeCode(initialCode);
    }

    // Listen for messages from sandboxed iframe
    window.addEventListener('message', (event) => {
      if (event.data.type === 'sandboxReady') {
        isReady = true;
        // Execute initial code once both editor and sandbox are ready
        if (editorReady) {
          executeCode(initialCode);
        }
      }
    });

    // Set up resizer (linkhash style)
    setupResizer();
  });

  async function loadMonaco() {
    return new Promise((resolve) => {
      const loaderScript = document.createElement('script');
      loaderScript.src = 'https://unpkg.com/monaco-editor@latest/min/vs/loader.js';
      loaderScript.onload = () => {
        window.require.config({
          paths: { vs: 'https://unpkg.com/monaco-editor@latest/min/vs' }
        });

        window.require(['vs/editor/editor.main'], () => {
          // Configure TypeScript compiler options to support top-level await
          monaco.languages.typescript.typescriptDefaults.setCompilerOptions({
            target: monaco.languages.typescript.ScriptTarget.ES2020,
            module: monaco.languages.typescript.ModuleKind.ESNext,
            moduleResolution: monaco.languages.typescript.ModuleResolutionKind.NodeJs,
            // Include all necessary type libraries
            lib: [
              'ES2015',
              'ES2016',
              'ES2017',
              'ES2018',
              'ES2019',
              'ES2020',
              'ESNext'
            ],
            allowNonTsExtensions: true,
            noSemanticValidation: false,
            noSyntaxValidation: false
          });

          // Add tana:core type definitions to Monaco
          monaco.languages.typescript.typescriptDefaults.addExtraLib(
            `declare module 'tana:core' {
              export const console: {
                log(...args: unknown[]): void;
                error(...args: unknown[]): void;
              };
              export const version: {
                tana: string;
                deno_core: string;
                v8: string;
              };
            }`,
            'ts:filename/tana-core.d.ts'
          );

          // Add tana:utils type definitions to Monaco
          monaco.languages.typescript.typescriptDefaults.addExtraLib(
            `declare module 'tana:utils' {
              /**
               * Whitelisted fetch API for Tana Playground
               *
               * Follows the standard Fetch API spec, but only allows requests to:
               * - pokeapi.co (testing)
               * - *.tana.dev (Tana infrastructure)
               * - localhost / 127.0.0.1 (local development)
               *
               * @param url - The URL to fetch from
               * @param options - Optional fetch options (method, headers, body, etc.)
               * @returns Promise that resolves to a Response
               * @throws Error if domain is not whitelisted
               *
               * @example
               * const response = await fetch('https://pokeapi.co/api/v2/pokemon/ditto');
               * const data = await response.json();
               * console.log(data);
               */
              export function fetch(
                url: string | URL,
                options?: RequestInit
              ): Promise<Response>;
            }`,
            'ts:filename/tana-utils.d.ts'
          );

          // Add tana:data type definitions to Monaco
          monaco.languages.typescript.typescriptDefaults.addExtraLib(
            `declare module 'tana:data' {
              export const data: {
                readonly MAX_KEY_SIZE: 256;
                readonly MAX_VALUE_SIZE: 10240;
                readonly MAX_TOTAL_SIZE: 102400;
                readonly MAX_KEYS: 1000;

                set(key: string, value: string | object): Promise<void>;
                get(key: string): Promise<string | object | null>;
                delete(key: string): Promise<void>;
                has(key: string): Promise<boolean>;
                keys(pattern?: string): Promise<string[]>;
                entries(): Promise<Record<string, string | object>>;
                clear(): Promise<void>;
                commit(): Promise<void>;
              };
            }`,
            'ts:filename/tana-data.d.ts'
          );

          // Add standard JavaScript globals (explicit declarations for sandboxed environment)
          monaco.languages.typescript.typescriptDefaults.addExtraLib(
            `
            // Standard JavaScript global functions
            declare function parseInt(string: string, radix?: number): number;
            declare function parseFloat(string: string): number;
            declare function isNaN(number: number): boolean;
            declare function isFinite(number: number): boolean;

            // Standard constructor types are already in ES2020 lib:
            // String, Number, Boolean, Date, Math, JSON, Array, Object, etc.
            `,
            'ts:filename/globals.d.ts'
          );

          editor = monaco.editor.create(editorContainer, {
            value: initialCode,
            language: 'typescript',
            theme: 'vs-dark',
            automaticLayout: true,
            minimap: { enabled: false },
            fontSize: 14,
            lineNumbers: 'on',
            scrollBeyondLastLine: false,
            wordWrap: 'on',
          });

          // Auto-execute on change (linkhash style) + update URL
          editor.onDidChangeModelContent(() => {
            const code = editor.getValue();
            debouncedUpdateUrl(code);
            debouncedExecute(code);
          });

          // Add Ctrl/Cmd+Enter to immediately execute without debounce
          editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter, () => {
            executeCode(editor.getValue());
          });

          resolve();
        });
      };
      document.head.appendChild(loaderScript);
    });
  }

  function setupResizer() {
    const leftPanel = editorContainer.parentElement;
    const rightPanel = outputContainer.parentElement;
    const resizer = resizerElem;

    function updatePanelSizes(clientX, clientY) {
      const containerRect = resizer.parentNode.getBoundingClientRect();

      if (window.innerWidth >= 1024) {
        // Horizontal resize (desktop)
        const minPanelWidth = containerRect.width * 0.3;
        let leftWidth = clientX - containerRect.left;
        let rightWidth = containerRect.width - leftWidth - resizer.offsetWidth;

        if (leftWidth < minPanelWidth) {
          leftWidth = minPanelWidth;
          rightWidth = containerRect.width - leftWidth - resizer.offsetWidth;
        } else if (rightWidth < minPanelWidth) {
          rightWidth = minPanelWidth;
          leftWidth = containerRect.width - rightWidth - resizer.offsetWidth;
        }

        leftPanel.style.width = `${leftWidth}px`;
        rightPanel.style.width = `${rightWidth}px`;
        leftPanel.style.height = '100%';
        rightPanel.style.height = '100%';
      } else {
        // Vertical resize (mobile)
        const minPanelHeight = containerRect.height * 0.3;
        let topHeight = clientY - containerRect.top;
        let bottomHeight = containerRect.height - topHeight - resizer.offsetHeight;

        if (topHeight < minPanelHeight) {
          topHeight = minPanelHeight;
          bottomHeight = containerRect.height - topHeight - resizer.offsetHeight;
        } else if (bottomHeight < minPanelHeight) {
          bottomHeight = minPanelHeight;
          topHeight = containerRect.height - bottomHeight - resizer.offsetHeight;
        }

        leftPanel.style.height = `${topHeight}px`;
        rightPanel.style.height = `${bottomHeight}px`;
        leftPanel.style.width = '100%';
        rightPanel.style.width = '100%';
      }
    }

    function startDragging(e) {
      isDragging = true;
      e.preventDefault();
    }

    resizer.addEventListener('mousedown', startDragging);
    resizer.addEventListener('touchstart', startDragging);

    function onMouseMove(e) {
      if (!isDragging) return;
      let clientX = e.clientX || e.touches?.[0]?.clientX;
      let clientY = e.clientY || e.touches?.[0]?.clientY;
      updatePanelSizes(clientX, clientY);
    }

    document.addEventListener('mousemove', onMouseMove);
    document.addEventListener('touchmove', onMouseMove);

    function stopDragging() {
      isDragging = false;
    }

    document.addEventListener('mouseup', stopDragging);
    document.addEventListener('touchend', stopDragging);

    function adjustLayout() {
      const lgBreakpoint = 1024;

      if (window.innerWidth >= lgBreakpoint) {
        leftPanel.style.height = '100%';
        rightPanel.style.height = '100%';
        resizer.style.width = '5px';
        resizer.style.height = '100%';
      } else {
        leftPanel.style.width = '100%';
        rightPanel.style.width = '100%';
        resizer.style.width = '100%';
        resizer.style.height = '5px';
      }
    }

    adjustLayout();

    window.addEventListener('resize', adjustLayout);
  }
</script>

<div class="flex flex-col lg:flex-row w-screen h-screen relative">
  <!-- Accent bar (linkhash style) -->
  <div class="accent-bar">
    <div class="accent-left"></div>
    <div class="accent-right"></div>
  </div>

  <!-- Editor Panel -->
  <div class="editor-panel">
    <div bind:this={editorContainer} class="editor-container"></div>
  </div>

  <!-- Resizer -->
  <div bind:this={resizerElem} class="resizer"></div>

  <!-- Output Panel -->
  <div bind:this={outputContainer} class="output-panel">
    <iframe
      bind:this={sandboxIframe}
      src="/sandbox"
      sandbox="allow-scripts"
      title="Tana Sandbox"
      class="sandbox-iframe"
    ></iframe>
  </div>
</div>

<style>
  /* Linkhash-inspired styling */
  :global(html, body) {
    width: 100%;
    height: 100%;
    overflow: hidden;
    margin: 0;
    padding: 0;
    background: #13151a;
  }

  .accent-bar {
    pointer-events: none;
    max-width: 20px;
    height: 100vh;
    position: absolute;
    z-index: 10;
    width: 20px;
  }

  .accent-left {
    height: 100vh;
    width: 50%;
    background: #ffaff3;
    float: left;
  }

  .accent-right {
    height: 100vh;
    width: 50%;
    background: rgb(245 208 254);
    float: left;
  }

  .editor-panel {
    flex-grow: 1;
    position: relative;
    margin-left: 20px;
    width: calc(50% - 10px);
    height: 50%;
    display: flex;
    flex-direction: column;
  }

  @media (min-width: 1024px) {
    .editor-panel {
      height: 100vh;
      width: 50%;
    }
  }

  .output-panel {
    flex-grow: 1;
    position: relative;
    width: 100%;
    height: 49%;
    display: flex;
    flex-direction: column;
    background-color: #fffbe8;
    color: #1e1e1e;
  }

  @media (min-width: 1024px) {
    .output-panel {
      width: calc(50% - 25px);
      height: 100vh;
    }
  }

  .resizer {
    background-color: #584355;
    cursor: ns-resize;
    height: 5px;
    width: 100%;
  }

  @media (min-width: 1024px) {
    .resizer {
      cursor: ew-resize;
      width: 5px;
      height: 100vh;
    }
  }

  .editor-container {
    flex: 1;
    overflow: hidden;
  }

  .sandbox-iframe {
    flex: 1;
    width: 100%;
    height: 100%;
    border: none;
    margin: 0;
    padding: 0;
  }
</style>
