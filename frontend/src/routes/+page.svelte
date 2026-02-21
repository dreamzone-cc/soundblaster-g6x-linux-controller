<script>
    // @ts-nocheck
    import { onMount } from "svelte";
    import { get } from "svelte/store";
    import { device } from "$lib/store";

    // Icons
    import {
        Monitor,
        Music,
        Volume2,
        VolumeX,
        Settings,
        Power,
        Headphones,
        Speaker,
        Mic,
        MicOff,
        SlidersHorizontal,
        Orbit,
        Sparkles,
        Drum,
        Activity,
        MessageSquare,
        ShieldBan,
        Save,
        Trash2,
        Plus,
        X,
    } from "lucide-svelte";

    let activeTab = "sbx";

    onMount(() => {
        device.startPolling(1500); // Poll every 1.5 seconds
        return () => device.stopPolling();
    });

    $: sbxEnabled =
        $device?.features?.find((f) => f.name === "SBX")?.value?.Toggle ||
        false;
    $: scoutEnabled =
        $device?.features?.find((f) => f.name === "Scout Mode")?.value
            ?.Toggle || false;

    // Feature Icon mapping
    const featureIcons = {
        Surround: Orbit,
        Crystalizer: Sparkles,
        Bass: Drum,
        "Smart Volume": Activity,
        "Dialog+": MessageSquare,
    };
    // Helper to get feature
    $: getFeature = (name) => $device?.features?.find((f) => f.name === name);
    $: getSlider = (name) => {
        const f = getFeature(name + " Slider");
        return f?.value?.Slider || 0;
    };

    function toggleFeature(name) {
        const state = get(device);
        const f = state?.features?.find((ft) => ft.name === name);
        if (!f) {
            console.warn("toggleFeature: feature not found:", name);
            return;
        }
        const current = f.value?.Toggle ?? false;
        console.log("toggleFeature:", name, current, "->", !current);
        device.updateFeature(name, !current);
    }

    function updateSlider(name, value) {
        // Smart Volume Special is a feature on its own, not "X Slider"
        if (name === "Smart Volume Special") {
            device.updateFeature("Smart Volume Special", value);
        } else {
            device.updateFeature(name + " Slider", value);
        }
    }

    // EQ Canvas rendering
    let eqCanvas;

    const eqBandsList = [
        "EQ Pre-Amp",
        "EQ 31Hz",
        "EQ 62Hz",
        "EQ 125Hz",
        "EQ 250Hz",
        "EQ 500Hz",
        "EQ 1kHz",
        "EQ 2kHz",
        "EQ 4kHz",
        "EQ 8kHz",
        "EQ 16kHz",
    ];

    function drawEQ() {
        if (!eqCanvas || activeTab !== "eq") return;
        const eqCtx = eqCanvas.getContext("2d");

        // Use device pixel ratio for sharp rendering
        const dpr = 1; // Simplify to avoid infinite scaling on some setups
        const rect = eqCanvas.getBoundingClientRect();

        if (rect.width === 0 || rect.height === 0) return;

        if (eqCanvas.width !== rect.width || eqCanvas.height !== rect.height) {
            eqCanvas.width = rect.width;
            eqCanvas.height = rect.height;
        }

        const w = rect.width;
        const h = rect.height;

        eqCtx.clearRect(0, 0, w, h);

        // Grid background
        eqCtx.strokeStyle = "#333";
        eqCtx.lineWidth = 1;

        for (let i = 0; i <= 4; i++) {
            let y = (h / 4) * i;
            eqCtx.beginPath();
            eqCtx.moveTo(0, y);
            eqCtx.lineTo(w, y);
            eqCtx.stroke();
        }

        // Draw FFT Spectrum if enabled
        if (analyserEnabled && analyser && dataArray) {
            analyser.getByteFrequencyData(dataArray);

            const barWidth = (w / dataArray.length) * 2.5;
            let xPos = 0;

            for (let i = 0; i < dataArray.length; i++) {
                const barHeight = (dataArray[i] / 255) * h;

                // Draw warm glowing bars matching the crimson theme
                eqCtx.fillStyle = `rgba(220, 38, 38, ${Math.max(0.1, dataArray[i] / 500)})`;
                eqCtx.fillRect(xPos, h - barHeight, barWidth, barHeight);
                xPos += barWidth + 1;
            }
        }

        // Gather points
        const points = eqBandsList.map((bandStr, i) => {
            const val =
                eqLocalValues[bandStr] !== undefined
                    ? eqLocalValues[bandStr]
                    : getFeature(bandStr)?.value?.Slider || 0;
            let x = (i / (eqBandsList.length - 1)) * w;
            let normalized = (val + 12) / 24;
            let y = h - normalized * h;
            return { x, y };
        });

        // Draw curve: cubic bezier smoothing
        eqCtx.beginPath();
        eqCtx.strokeStyle = "#ef4444";
        eqCtx.lineWidth = 3;

        if (points.length > 0) {
            eqCtx.moveTo(points[0].x, points[0].y);

            for (let i = 0; i < points.length - 1; i++) {
                const tension = 0.2;
                const p0 = i === 0 ? points[0] : points[i - 1];
                const p1 = points[i];
                const p2 = points[i + 1];
                const p3 =
                    i === points.length - 2 ? points[i + 1] : points[i + 2];

                const cp1x = p1.x + (p2.x - p0.x) * tension;
                const cp1y = p1.y + (p2.y - p0.y) * tension;

                const cp2x = p2.x - (p3.x - p1.x) * tension;
                const cp2y = p2.y - (p3.y - p1.y) * tension;

                eqCtx.bezierCurveTo(cp1x, cp1y, cp2x, cp2y, p2.x, p2.y);
            }
        }

        eqCtx.stroke();

        // Draw points
        eqBandsList.forEach((bandStr, i) => {
            const val =
                eqLocalValues[bandStr] !== undefined
                    ? eqLocalValues[bandStr]
                    : getFeature(bandStr)?.value?.Slider || 0;

            let x = (i / (eqBandsList.length - 1)) * w;
            let normalized = (val + 12) / 24;
            let y = h - normalized * h;

            eqCtx.beginPath();
            eqCtx.fillStyle = "#ef4444";
            eqCtx.arc(x, y, 6, 0, Math.PI * 2);
            eqCtx.fill();

            eqCtx.strokeStyle = "#111";
            eqCtx.lineWidth = 2;
            eqCtx.stroke();
        });
    }

    // Spectrum Analyzer
    let analyserEnabled = false;
    let audioContext;
    let analyser;
    let dataArray;
    let animationId;

    async function toggleAnalyzer() {
        if (analyserEnabled) {
            analyserEnabled = false;
            if (audioContext) {
                audioContext.close();
                audioContext = null;
            }
            if (animationId) cancelAnimationFrame(animationId);
            drawEQ(); // draw final static frame
            return;
        }

        try {
            const stream = await navigator.mediaDevices.getUserMedia({
                audio: true,
            });
            audioContext = new (window.AudioContext ||
                window.webkitAudioContext)();
            const source = audioContext.createMediaStreamSource(stream);
            analyser = audioContext.createAnalyser();
            analyser.fftSize = 64; // fewer bins for a cleaner visual
            source.connect(analyser);

            const bufferLength = analyser.frequencyBinCount;
            dataArray = new Uint8Array(bufferLength);
            analyserEnabled = true;
            renderLoop();
        } catch (err) {
            console.error("Microphone access denied or not available", err);
            alert("Could not access microphone for visualization.");
        }
    }

    function renderLoop() {
        if (!analyserEnabled) return;
        drawEQ();
        animationId = requestAnimationFrame(renderLoop);
    }

    let isDraggingSlider = false;
    let eqLocalValues = {};

    // Reactively sync store to local only when not dragging
    $: {
        if (
            $device?.features &&
            draggingBandIndex === -1 &&
            !isDraggingSlider
        ) {
            let nextLocal = { ...eqLocalValues };
            let changed = false;
            $device.features.forEach((f) => {
                if (f.name.startsWith("EQ")) {
                    if (nextLocal[f.name] !== f.value?.Slider) {
                        nextLocal[f.name] = f.value?.Slider || 0;
                        changed = true;
                    }
                }
            });
            if (changed) eqLocalValues = nextLocal;
        }
    }

    // Reactively redraw whenever the store, tab, or eqLocalValues change (if not running loop)
    $: {
        if (
            activeTab === "eq" &&
            eqCanvas &&
            ($device || eqLocalValues) &&
            !analyserEnabled
        ) {
            // Need a tiny delay for layout to settle if component just mounted
            setTimeout(drawEQ, 50);
        }
    }

    // Dragging logic for EQ Canvas
    let draggingBandIndex = -1;

    function handleCanvasDown(e) {
        if (
            !eqCanvas ||
            activeTab !== "eq" ||
            !getFeature("Equalizer")?.value?.Toggle
        )
            return;
        const rect = eqCanvas.getBoundingClientRect();
        const x = e.clientX - rect.left;
        const y = e.clientY - rect.top;
        const w = rect.width;
        const h = rect.height;

        for (let i = 0; i < eqBandsList.length; i++) {
            const val = getFeature(eqBandsList[i])?.value?.Slider || 0;
            let nodeX = (i / (eqBandsList.length - 1)) * w;
            let normalized = (val + 12) / 24;
            let nodeY = h - normalized * h;

            // Check if mouse is near this node (20px tolerance)
            if (Math.abs(x - nodeX) < 20 && Math.abs(y - nodeY) < 20) {
                draggingBandIndex = i;
                break;
            }
        }
    }

    function handleCanvasMove(e) {
        if (
            draggingBandIndex === -1 ||
            !eqCanvas ||
            !getFeature("Equalizer")?.value?.Toggle
        )
            return;
        const rect = eqCanvas.getBoundingClientRect();
        const y = e.clientY - rect.top;
        const h = rect.height;

        let normalized = 1.0 - y / h;
        let val = normalized * 24 - 12;

        // Snap to step 0.5
        val = Math.round(val * 2) / 2;
        if (val > 12) val = 12;
        if (val < -12) val = -12;

        const bandStr = eqBandsList[draggingBandIndex];

        // Ensure real-time local updates and Canvas draws immediately
        eqLocalValues[bandStr] = val;
        device.updateFeature(bandStr, val);
    }

    function handleCanvasUp() {
        draggingBandIndex = -1;
    }

    // EQ Presets - built-in + user-saved from localStorage
    const builtInPresets = {
        Flat: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        "Bass Boost": [0, 5, 4, 3, 1, -1, -2, 0, 1, 2, 3],
        Acoustic: [0, 4, 4, 2, 0, -1, 1, 2, 3, 4, 3],
        EDM: [0, 5, 4, 2, 0, -2, 0, 2, 4, 5, 6],
        Gaming: [0, 4, 2, -2, 0, 2, 4, 6, 8, 6, 4],
    };

    // Load user presets from localStorage
    function loadUserPresets() {
        try {
            const saved = localStorage.getItem("eq_user_presets");
            return saved ? JSON.parse(saved) : {};
        } catch {
            return {};
        }
    }
    function saveUserPresetsToStorage(presets) {
        try {
            localStorage.setItem("eq_user_presets", JSON.stringify(presets));
        } catch {}
    }

    let userPresets = loadUserPresets();
    $: eqPresets = { ...builtInPresets, ...userPresets };
    let eqSelectedPreset = "Custom";
    let showSaveDialog = false;
    let newPresetName = "";

    function applyPreset(event) {
        const presetName = event.target.value;
        eqSelectedPreset = presetName;
        if (presetName === "Custom" || !eqPresets[presetName]) return;
        const vals = eqPresets[presetName];
        eqBandsList.forEach((bandStr, idx) => {
            eqLocalValues[bandStr] = vals[idx];
            device.updateFeature(bandStr, vals[idx]);
        });
    }

    function openSaveDialog() {
        newPresetName = "";
        showSaveDialog = true;
    }

    function saveCustomPreset() {
        const name = newPresetName.trim();
        if (!name) return;
        // Gather current EQ values
        const vals = eqBandsList.map((bandStr) => {
            return eqLocalValues[bandStr] !== undefined
                ? eqLocalValues[bandStr]
                : getFeature(bandStr)?.value?.Slider || 0;
        });
        userPresets = { ...userPresets, [name]: vals };
        saveUserPresetsToStorage(userPresets);
        eqSelectedPreset = name;
        showSaveDialog = false;
        newPresetName = "";
    }

    function deleteCurrentPreset() {
        if (!eqSelectedPreset || eqSelectedPreset === "Custom") return;
        if (builtInPresets[eqSelectedPreset]) return; // Can't delete built-in
        const { [eqSelectedPreset]: _, ...rest } = userPresets;
        userPresets = rest;
        saveUserPresetsToStorage(userPresets);
        eqSelectedPreset = "Custom";
    }

    $: isCustomPresetSelected =
        eqSelectedPreset !== "Custom" &&
        !builtInPresets[eqSelectedPreset] &&
        userPresets[eqSelectedPreset];
</script>

<div
    class="flex flex-col h-screen bg-[#111] text-gray-200 font-sans overflow-hidden select-none"
>
    <!-- Top Layout Area -->
    <div class="flex flex-1 overflow-hidden w-full h-full">
        <!-- Sidebar -->
        <aside
            class="w-64 shrink-0 bg-[#1a1a1a] flex flex-col border-r border-[#2a2a2a] z-10 relative hidden md:flex"
        >
            <!-- Logo Header -->
            <div
                class="p-5 flex flex-col items-center border-b border-[#2a2a2a]"
            >
                <img
                    src="/logo.svg"
                    alt="Sound Blaster G6X"
                    class="w-32 h-auto mb-2 drop-shadow-lg"
                />
                <h1
                    class="font-bold text-sm tracking-widest text-gray-400 uppercase"
                >
                    G6X Controller
                </h1>
            </div>

            <!-- Navigation -->
            <nav class="flex-1 p-4 space-y-2">
                <button
                    class="w-full flex items-center space-x-3 px-4 py-3 rounded-lg transition-all duration-200 {activeTab ===
                    'sbx'
                        ? 'bg-red-600/10 text-red-500 shadow-sm border border-red-500/20'
                        : 'hover:bg-[#252525] text-gray-400'}"
                    on:click={() => (activeTab = "sbx")}
                >
                    <Monitor size={20} />
                    <span class="font-medium">SBX Profile</span>
                </button>
                <button
                    class="w-full flex items-center space-x-3 px-4 py-3 rounded-lg transition-all duration-200 {activeTab ===
                    'eq'
                        ? 'bg-red-600/10 text-red-500 shadow-sm border border-red-500/20'
                        : 'hover:bg-[#252525] text-gray-400'}"
                    on:click={() => (activeTab = "eq")}
                >
                    <Settings size={20} />
                    <span class="font-medium">Equalizer</span>
                </button>
                <button
                    class="w-full flex items-center space-x-3 px-4 py-3 rounded-lg transition-all duration-200 {activeTab ===
                    'mixer'
                        ? 'bg-red-600/10 text-red-500 shadow-sm border border-red-500/20'
                        : 'hover:bg-[#252525] text-gray-400'}"
                    on:click={() => (activeTab = "mixer")}
                >
                    <Volume2 size={20} />
                    <span class="font-medium">Mixer</span>
                </button>
            </nav>

            <!-- Branding Logos -->
            <div class="border-t border-[#2a2a2a] p-4 space-y-4">
                <div
                    class="flex items-center justify-center px-3 py-2 opacity-70 hover:opacity-100 transition-opacity duration-300"
                >
                    <img
                        src="/SoundBlaster.svg"
                        alt="Sound Blaster"
                        class="w-full max-w-[180px] h-auto"
                    />
                </div>
                <div
                    class="flex items-center justify-center px-3 py-2 opacity-70 hover:opacity-100 transition-opacity duration-300"
                >
                    <img
                        src="/yuuyu.svg"
                        alt="Yuuyu"
                        class="w-full max-w-[140px] h-auto"
                    />
                </div>
                <div class="flex items-center justify-center gap-6 px-3 py-2">
                    <div
                        class="opacity-60 hover:opacity-100 transition-opacity duration-300"
                    >
                        <img src="/dzz.png" alt="DZZ" class="h-10 w-auto" />
                    </div>
                    <div
                        class="opacity-60 hover:opacity-100 transition-opacity duration-300"
                    >
                        <img src="/1337.png" alt="1337" class="h-10 w-auto" />
                    </div>
                </div>
            </div>

            <!-- Developer Info -->
            <div class="border-t border-[#2a2a2a] px-4 py-3">
                <div
                    class="text-[9px] uppercase tracking-widest text-gray-600 font-bold mb-2"
                >
                    Developer
                </div>
                <a
                    href="https://github.com/dreamzone-cc"
                    target="_blank"
                    rel="noopener noreferrer"
                    class="flex items-center space-x-2 px-2 py-1.5 rounded-md text-gray-500 hover:text-white hover:bg-[#252525] transition-all text-xs"
                >
                    <svg
                        class="w-3.5 h-3.5"
                        viewBox="0 0 24 24"
                        fill="currentColor"
                        ><path
                            d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0024 12c0-6.63-5.37-12-12-12z"
                        /></svg
                    >
                    <span>dreamzone-cc</span>
                </a>
                <div
                    class="flex items-center space-x-2 px-2 py-1.5 text-gray-500 text-xs"
                >
                    <svg
                        class="w-3.5 h-3.5"
                        viewBox="0 0 24 24"
                        fill="currentColor"
                        ><path
                            d="M20.317 4.37a19.791 19.791 0 00-4.885-1.515.074.074 0 00-.079.037c-.21.375-.444.864-.608 1.25a18.27 18.27 0 00-5.487 0 12.64 12.64 0 00-.617-1.25.077.077 0 00-.079-.037A19.736 19.736 0 003.677 4.37a.07.07 0 00-.032.027C.533 9.046-.32 13.58.099 18.057a.082.082 0 00.031.057 19.9 19.9 0 005.993 3.03.078.078 0 00.084-.028c.462-.63.874-1.295 1.226-1.994a.076.076 0 00-.041-.106 13.107 13.107 0 01-1.872-.892.077.077 0 01-.008-.128c.126-.094.252-.192.372-.291a.074.074 0 01.077-.01c3.928 1.793 8.18 1.793 12.062 0a.074.074 0 01.078.01c.12.098.246.198.373.292a.077.077 0 01-.006.127 12.299 12.299 0 01-1.873.892.077.077 0 00-.041.107c.36.698.772 1.362 1.225 1.993a.076.076 0 00.084.028 19.839 19.839 0 006.002-3.03.077.077 0 00.032-.054c.5-5.177-.838-9.674-3.549-13.66a.061.061 0 00-.031-.03zM8.02 15.33c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.956-2.419 2.157-2.419 1.21 0 2.176 1.095 2.157 2.42 0 1.333-.956 2.418-2.157 2.418zm7.975 0c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.955-2.419 2.157-2.419 1.21 0 2.176 1.095 2.157 2.42 0 1.333-.946 2.418-2.157 2.418z"
                        /></svg
                    >
                    <span>yuuyu_gg</span>
                </div>
            </div>
        </aside>

        <!-- Main Content -->
        <main
            class="flex-1 overflow-y-auto flex flex-col bg-gradient-to-br from-[#111] to-[#0f0f0f] relative w-full h-full pb-8"
        >
            <!-- Background Ambient Glow -->
            <div
                class="absolute top-0 right-0 w-[500px] h-[500px] bg-red-600/5 rounded-full blur-[120px] pointer-events-none"
            ></div>

            <header
                class="p-8 pb-4 flex justify-between items-center relative z-10"
            >
                <div>
                    <h2
                        class="text-3xl font-bold bg-clip-text text-transparent bg-gradient-to-r from-white to-gray-500"
                    >
                        {activeTab === "sbx"
                            ? "Acoustic Engine"
                            : activeTab === "eq"
                              ? "Equalizer"
                              : "Audio Mixer"}
                    </h2>
                    <p class="text-gray-500 mt-1">
                        Configure your Sound BlasterX G6 experience
                    </p>
                </div>

                <!-- Output Mode Switcher -->
                {#if getFeature("Output Mode")}
                    {@const outputMode = getFeature("Output Mode")}
                    <button
                        class="flex items-center bg-[#252525] border border-[#333] rounded-full p-1 cursor-pointer hover:border-[#444] transition-all focus:outline-none focus:ring-2 focus:ring-red-500/50 object-contain shadow-lg"
                        on:click={() => toggleFeature("Output Mode")}
                        title={outputMode?.value?.Toggle
                            ? "Switch to Speakers"
                            : "Switch to Headphones"}
                    >
                        <div
                            class="px-4 py-2 rounded-full flex items-center space-x-2 transition-colors {outputMode
                                ?.value?.Toggle
                                ? 'bg-red-600/20 text-red-500 font-bold'
                                : 'text-gray-500 hover:text-gray-300'}"
                        >
                            <Headphones size={20} />
                            <span class="text-sm">Headphones</span>
                        </div>
                        <div
                            class="px-4 py-2 rounded-full flex items-center space-x-2 transition-colors {!outputMode
                                ?.value?.Toggle
                                ? 'bg-red-600/20 text-red-500 font-bold'
                                : 'text-gray-500 hover:text-gray-300'}"
                        >
                            <Speaker size={20} />
                            <span class="text-sm">Speakers</span>
                        </div>
                    </button>
                {/if}
            </header>

            <div class="p-8 pt-4">
                {#if $device.loading}
                    <div class="flex items-center justify-center h-64">
                        <div
                            class="animate-spin rounded-full h-12 w-12 border-b-2 border-red-600"
                        ></div>
                    </div>
                {:else if activeTab === "sbx"}
                    <!-- SBX Controls -->
                    <div class="space-y-6">
                        <!-- Master SBX Switch Card -->
                        <div
                            class="bg-gradient-to-r from-[#1a1a1a] via-[#1d1010] to-[#1a1a1a] border border-red-500/20 rounded-2xl p-8 flex items-center justify-between shadow-xl relative overflow-hidden"
                        >
                            <div
                                class="absolute inset-0 bg-[url('/noise.png')] opacity-[0.03] pointer-events-none"
                            ></div>
                            <div
                                class="absolute top-0 left-0 w-full h-1 bg-gradient-to-r from-transparent via-red-600 to-transparent opacity-60"
                            ></div>
                            <div
                                class="flex items-center space-x-5 relative z-10"
                            >
                                <div
                                    class="p-4 bg-red-600/10 rounded-xl text-red-500 shadow-[0_0_20px_rgba(220,38,38,0.15)]"
                                >
                                    <Power size={28} />
                                </div>
                                <div>
                                    <h3
                                        class="font-bold text-white text-xl tracking-tight flex items-center gap-3"
                                    >
                                        SBX Acoustic Engine
                                        {#if sbxEnabled}
                                            <span
                                                class="text-[10px] uppercase font-bold bg-green-600/20 text-green-400 px-2 py-0.5 rounded border border-green-500/30 animate-pulse"
                                                >Active</span
                                            >
                                        {:else}
                                            <span
                                                class="text-[10px] uppercase font-bold bg-gray-700/50 text-gray-500 px-2 py-0.5 rounded border border-gray-600/30"
                                                >Offline</span
                                            >
                                        {/if}
                                    </h3>
                                    <p
                                        class="text-sm text-gray-500 mt-0.5 max-w-lg"
                                    >
                                        Master switch controls the Sound Blaster
                                        audio processing pipeline. All SBX
                                        features (Surround, Crystalizer, Bass,
                                        etc.) require this to be enabled.
                                    </p>
                                </div>
                            </div>
                            <button
                                class="relative inline-flex h-10 w-[4.5rem] items-center rounded-full transition-all duration-300 focus:outline-none focus:ring-2 focus:ring-red-500 focus:ring-offset-2 focus:ring-offset-[#1a1a1a] shadow-lg z-10 {sbxEnabled
                                    ? 'bg-red-600 shadow-[0_0_20px_rgba(220,38,38,0.5)]'
                                    : 'bg-gray-700'}"
                                on:click={() => toggleFeature("SBX")}
                            >
                                <span class="sr-only">Enable SBX</span>
                                <span
                                    class="inline-block h-8 w-8 transform rounded-full bg-white transition-transform shadow-md {sbxEnabled
                                        ? 'translate-x-8'
                                        : 'translate-x-1'}"
                                ></span>
                            </button>
                        </div>

                        <!-- Scout Mode Card -->
                        <div
                            class="bg-gradient-to-r from-[#1a1a1a] via-[#101420] to-[#1a1a1a] border {scoutEnabled
                                ? 'border-blue-500/40 shadow-[0_0_25px_rgba(59,130,246,0.1)]'
                                : 'border-[#2a2a2a]'} rounded-2xl p-8 flex items-center justify-between shadow-lg relative overflow-hidden transition-all duration-500"
                        >
                            <div
                                class="absolute inset-0 bg-[url('/noise.png')] opacity-[0.03] pointer-events-none"
                            ></div>
                            {#if scoutEnabled}
                                <div
                                    class="absolute top-0 left-0 w-full h-1 bg-gradient-to-r from-transparent via-blue-500 to-transparent opacity-60"
                                ></div>
                            {/if}
                            <div
                                class="flex items-center space-x-5 relative z-10"
                            >
                                <div
                                    class="p-4 {scoutEnabled
                                        ? 'bg-blue-500/15 text-blue-400 shadow-[0_0_20px_rgba(59,130,246,0.15)]'
                                        : 'bg-blue-600/10 text-blue-600'} rounded-xl transition-all duration-300"
                                >
                                    <Monitor size={28} />
                                </div>
                                <div>
                                    <h3
                                        class="font-bold text-white text-xl tracking-tight flex items-center gap-3"
                                    >
                                        Scout Mode
                                        {#if scoutEnabled}
                                            <span
                                                class="text-[10px] uppercase font-bold bg-blue-600/20 text-blue-400 px-2 py-0.5 rounded border border-blue-500/30"
                                                >Engaged</span
                                            >
                                        {/if}
                                    </h3>
                                    <p
                                        class="text-sm text-gray-500 mt-0.5 max-w-lg"
                                    >
                                        Enhances awareness in FPS games by
                                        amplifying footsteps, reloads, and
                                        ambient cues while reducing loud
                                        explosions. Disables SBX when active.
                                    </p>
                                </div>
                            </div>
                            <button
                                class="relative inline-flex h-10 w-[4.5rem] items-center rounded-full transition-all duration-300 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:ring-offset-[#1a1a1a] shadow-lg z-10 {scoutEnabled
                                    ? 'bg-blue-600 shadow-[0_0_20px_rgba(59,130,246,0.5)]'
                                    : 'bg-gray-700'}"
                                on:click={() => toggleFeature("Scout Mode")}
                            >
                                <span class="sr-only">Enable Scout Mode</span>
                                <span
                                    class="inline-block h-8 w-8 transform rounded-full bg-white transition-transform shadow-md {scoutEnabled
                                        ? 'translate-x-8'
                                        : 'translate-x-1'}"
                                ></span>
                            </button>
                        </div>

                        <!-- Feature Cards Grid -->
                        <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
                            {#each [{ name: "Surround", desc: "Virtual 7.1 surround sound immersion using Head-Related Transfer Function (HRTF) algorithms for precise positional audio.", accentFrom: "#7c3aed", accentTo: "#a855f7", tagColor: "purple" }, { name: "Crystalizer", desc: "Restores dynamic range and clarity lost during audio compression. Enhances hi-fi detail in music and media playback.", accentFrom: "#0891b2", accentTo: "#22d3ee", tagColor: "cyan" }, { name: "Bass", desc: "Digitally enriches low-frequency response for deeper, more impactful bass without distorting mids or highs.", accentFrom: "#ea580c", accentTo: "#fb923c", tagColor: "orange" }, { name: "Smart Volume", desc: "Intelligent loudness normalization. Reduces volume spikes and boosts quiet passages for a consistent listening level.", accentFrom: "#16a34a", accentTo: "#4ade80", tagColor: "green" }, { name: "Dialog+", desc: "Isolates and enhances voice frequencies in movies, podcasts, and voice chat for crystal-clear dialog reproduction.", accentFrom: "#d97706", accentTo: "#fbbf24", tagColor: "amber" }] as featureInfo}
                                {@const feature = getFeature(featureInfo.name)}
                                {@const sliderVal = getSlider(featureInfo.name)}
                                {#if feature}
                                    {@const isActive = feature.value.Toggle}
                                    {@const IconComponent =
                                        featureIcons[featureInfo.name]}
                                    <div
                                        class="bg-[#1a1a1a] border {isActive
                                            ? 'border-[#444] shadow-lg'
                                            : 'border-[#2a2a2a]'} rounded-2xl shadow-md hover:shadow-lg transition-all duration-300 group relative overflow-hidden"
                                    >
                                        <!-- Top Accent Bar -->
                                        <div
                                            class="h-1 w-full transition-opacity duration-500 {isActive
                                                ? 'opacity-100'
                                                : 'opacity-0'}"
                                            style="background: linear-gradient(90deg, {featureInfo.accentFrom}, {featureInfo.accentTo});"
                                        ></div>

                                        <div class="p-6">
                                            <!-- Visual Noise Texture -->
                                            <div
                                                class="absolute inset-0 bg-[url('/noise.png')] opacity-[0.02] pointer-events-none"
                                            ></div>

                                            <!-- Active Background Glow -->
                                            <div
                                                class="absolute inset-0 opacity-0 transition-opacity duration-500 {isActive
                                                    ? 'opacity-100'
                                                    : ''} pointer-events-none"
                                                style="background: radial-gradient(ellipse at top left, color-mix(in srgb, {featureInfo.accentFrom} 8%, transparent), transparent 70%);"
                                            ></div>

                                            <!-- Header Row -->
                                            <div
                                                class="flex items-center justify-between mb-4 relative z-10"
                                            >
                                                <div
                                                    class="flex items-center space-x-3"
                                                >
                                                    <div
                                                        class="w-10 h-10 rounded-xl flex items-center justify-center transition-all duration-300"
                                                        style="background: {isActive
                                                            ? `linear-gradient(135deg, color-mix(in srgb, ${featureInfo.accentFrom} 20%, transparent), color-mix(in srgb, ${featureInfo.accentTo} 10%, transparent))`
                                                            : '#252525'}; color: {isActive
                                                            ? featureInfo.accentTo
                                                            : '#666'}; {isActive
                                                            ? `box-shadow: 0 0 15px color-mix(in srgb, ${featureInfo.accentFrom} 25%, transparent)`
                                                            : ''}"
                                                    >
                                                        <svelte:component
                                                            this={IconComponent}
                                                            size={20}
                                                        />
                                                    </div>
                                                    <div>
                                                        <h3
                                                            class="font-bold {isActive
                                                                ? 'text-white'
                                                                : 'text-gray-300'} text-lg transition-colors"
                                                        >
                                                            {featureInfo.name}
                                                        </h3>
                                                    </div>
                                                </div>
                                                <button
                                                    class="relative inline-flex h-7 w-12 items-center rounded-full transition-all duration-300 focus:outline-none"
                                                    style="background: {isActive
                                                        ? featureInfo.accentFrom
                                                        : '#374151'}; {isActive
                                                        ? `box-shadow: 0 0 12px color-mix(in srgb, ${featureInfo.accentFrom} 50%, transparent)`
                                                        : ''}"
                                                    on:click={() =>
                                                        toggleFeature(
                                                            featureInfo.name,
                                                        )}
                                                >
                                                    <span class="sr-only"
                                                        >Toggle {featureInfo.name}</span
                                                    >
                                                    <span
                                                        class="inline-block h-5 w-5 transform rounded-full bg-white transition-transform shadow-sm {isActive
                                                            ? 'translate-x-6'
                                                            : 'translate-x-1'}"
                                                    ></span>
                                                </button>
                                            </div>

                                            <!-- Feature Description -->
                                            <p
                                                class="text-xs text-gray-500 mb-5 leading-relaxed relative z-10 pr-4"
                                            >
                                                {featureInfo.desc}
                                            </p>

                                            {#if isActive}
                                                <div
                                                    class="relative z-10 space-y-3 animate-in fade-in duration-300"
                                                >
                                                    <!-- Slider Labels -->
                                                    <div
                                                        class="flex justify-between text-[10px] uppercase tracking-wider font-bold mb-1"
                                                    >
                                                        <span
                                                            class="text-gray-600"
                                                            >Min</span
                                                        >
                                                        <span
                                                            style="color: {featureInfo.accentTo};"
                                                            >{(
                                                                sliderVal * 100
                                                            ).toFixed(0)}%</span
                                                        >
                                                        <span
                                                            class="text-gray-600"
                                                            >Max</span
                                                        >
                                                    </div>
                                                    <!-- Slider with custom accent -->
                                                    <div class="relative">
                                                        <div
                                                            class="absolute inset-y-0 left-0 h-2 rounded-full top-1/2 -translate-y-1/2 pointer-events-none transition-all duration-150"
                                                            style="width: {sliderVal *
                                                                100}%; background: linear-gradient(90deg, {featureInfo.accentFrom}, {featureInfo.accentTo}); opacity: 0.6;"
                                                        ></div>
                                                        <input
                                                            type="range"
                                                            min="0"
                                                            max="1"
                                                            step="0.01"
                                                            value={sliderVal}
                                                            on:input={(e) =>
                                                                updateSlider(
                                                                    featureInfo.name,
                                                                    parseFloat(
                                                                        e
                                                                            .currentTarget
                                                                            .value,
                                                                    ),
                                                                )}
                                                            class="w-full h-2 bg-gray-800 rounded-lg appearance-none cursor-pointer relative z-10"
                                                            style="accent-color: {featureInfo.accentTo};"
                                                        />
                                                    </div>

                                                    <!-- Smart Volume Night Mode -->
                                                    {#if featureInfo.name === "Smart Volume"}
                                                        {#if getFeature("Smart Volume Special")}
                                                            {@const nightMode =
                                                                getFeature(
                                                                    "Smart Volume Special",
                                                                )}
                                                            <div
                                                                class="flex items-center justify-between pt-4 mt-2 border-t border-green-500/10"
                                                            >
                                                                <div
                                                                    class="flex items-center space-x-3"
                                                                >
                                                                    <div
                                                                        class="w-7 h-7 rounded-lg flex items-center justify-center {nightMode
                                                                            .value
                                                                            .Slider >
                                                                        0.1
                                                                            ? 'bg-green-600/20 text-green-400'
                                                                            : 'bg-gray-800 text-gray-600'}"
                                                                    >
                                                                        <Monitor
                                                                            size={14}
                                                                        />
                                                                    </div>
                                                                    <div>
                                                                        <span
                                                                            class="text-sm {nightMode
                                                                                .value
                                                                                .Slider >
                                                                            0.1
                                                                                ? 'text-green-300'
                                                                                : 'text-gray-400'} font-medium"
                                                                            >Night
                                                                            Mode</span
                                                                        >
                                                                        <p
                                                                            class="text-[10px] text-gray-600"
                                                                        >
                                                                            Aggressively
                                                                            compresses
                                                                            dynamic
                                                                            range
                                                                            for
                                                                            quiet
                                                                            environments
                                                                        </p>
                                                                    </div>
                                                                </div>
                                                                <button
                                                                    class="relative inline-flex h-6 w-10 items-center rounded-full transition-all focus:outline-none {nightMode
                                                                        .value
                                                                        .Slider >
                                                                    0.1
                                                                        ? 'bg-green-600 shadow-[0_0_10px_rgba(22,163,74,0.4)]'
                                                                        : 'bg-gray-700'}"
                                                                    on:click={() =>
                                                                        updateSlider(
                                                                            "Smart Volume Special",
                                                                            nightMode
                                                                                .value
                                                                                .Slider >
                                                                                0.1
                                                                                ? 0.0
                                                                                : 1.0,
                                                                        )}
                                                                >
                                                                    <span
                                                                        class="inline-block h-4 w-4 transform rounded-full bg-white transition-transform {nightMode
                                                                            .value
                                                                            .Slider >
                                                                        0.1
                                                                            ? 'translate-x-5'
                                                                            : 'translate-x-1'}"
                                                                    ></span>
                                                                </button>
                                                            </div>
                                                        {/if}
                                                    {/if}
                                                </div>
                                            {:else}
                                                <div
                                                    class="h-16 flex items-center justify-center text-gray-700 text-sm relative z-10"
                                                >
                                                    <span
                                                        class="opacity-60 font-medium tracking-wide"
                                                        >Disabled</span
                                                    >
                                                </div>
                                            {/if}
                                        </div>
                                    </div>
                                {/if}
                            {/each}
                        </div>
                    </div>
                {:else if activeTab === "eq"}
                    <div
                        class="bg-[#1a1a1a] border border-[#2a2a2a] rounded-2xl p-8"
                    >
                        <div class="flex items-center justify-between mb-8">
                            <div class="flex items-center space-x-4">
                                <div
                                    class="p-3 bg-red-600/10 rounded-lg text-red-500"
                                >
                                    <Settings size={24} />
                                </div>
                                <div>
                                    <h3 class="font-bold text-white text-lg">
                                        Equalizer
                                    </h3>
                                    <p class="text-sm text-gray-500">
                                        Fine-tune frequencies
                                    </p>
                                </div>
                                <div
                                    class="ml-6 border-l border-[#2a2a2a] pl-6 hidden md:flex space-x-3 items-center"
                                >
                                    <div>
                                        <label for="eq_preset" class="sr-only"
                                            >Preset</label
                                        >
                                        <select
                                            id="eq_preset"
                                            class="bg-[#111] border border-[#2a2a2a] text-sm text-gray-200 rounded-lg focus:ring-red-500 focus:border-red-500 block w-full p-2 hover:border-[#3a3a3a] transition-colors focus:outline-none"
                                            style="color-scheme: dark;"
                                            bind:value={eqSelectedPreset}
                                            on:change={applyPreset}
                                        >
                                            <option value="Custom"
                                                >Custom</option
                                            >
                                            <optgroup label="Built-in">
                                                {#each Object.keys(builtInPresets) as presetKey}
                                                    <option value={presetKey}
                                                        >{presetKey}</option
                                                    >
                                                {/each}
                                            </optgroup>
                                            {#if Object.keys(userPresets).length > 0}
                                                <optgroup label="My Presets">
                                                    {#each Object.keys(userPresets) as presetKey}
                                                        <option
                                                            value={presetKey}
                                                            >⭐ {presetKey}</option
                                                        >
                                                    {/each}
                                                </optgroup>
                                            {/if}
                                        </select>
                                    </div>
                                    <!-- Save Preset Button -->
                                    <button
                                        class="px-3 py-2 bg-[#111] border border-[#2a2a2a] text-gray-400 text-xs font-semibold rounded-lg hover:border-green-500/50 hover:text-green-400 transition-all flex items-center space-x-1.5"
                                        on:click={openSaveDialog}
                                        title="Save current EQ as preset"
                                    >
                                        <Save size={14} />
                                        <span>Save</span>
                                    </button>
                                    <!-- Delete Preset Button (only for user presets) -->
                                    {#if isCustomPresetSelected}
                                        <button
                                            class="px-3 py-2 bg-[#111] border border-red-500/30 text-red-400 text-xs font-semibold rounded-lg hover:border-red-500/60 hover:bg-red-600/10 transition-all flex items-center space-x-1.5"
                                            on:click={deleteCurrentPreset}
                                            title="Delete '{eqSelectedPreset}' preset"
                                        >
                                            <Trash2 size={14} />
                                            <span>Delete</span>
                                        </button>
                                    {/if}
                                    <button
                                        class="px-3 py-2 bg-[#111] border {analyserEnabled
                                            ? 'border-red-500/50 text-red-500'
                                            : 'border-[#2a2a2a] text-gray-500'} text-xs font-semibold rounded-lg hover:border-[#3a3a3a] transition-all"
                                        on:click={toggleAnalyzer}
                                        title="Mic Spectrum Analyzer"
                                    >
                                        {analyserEnabled
                                            ? "🎤 FFT On"
                                            : "🎤 FFT Off"}
                                    </button>
                                </div>
                            </div>
                            <!-- EQ Toggle -->
                            {#if getFeature("Equalizer")}
                                {@const eqParams = getFeature("Equalizer")}
                                <button
                                    class="relative inline-flex h-8 w-14 items-center rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-red-500 focus:ring-offset-2 focus:ring-offset-[#1a1a1a] {eqParams
                                        ?.value?.Toggle
                                        ? 'bg-red-600'
                                        : 'bg-gray-700'}"
                                    on:click={() => toggleFeature("Equalizer")}
                                >
                                    <span class="sr-only">Enable Equalizer</span
                                    >
                                    <span
                                        class="inline-block h-6 w-6 transform rounded-full bg-white transition-transform {eqParams
                                            ?.value?.Toggle
                                            ? 'translate-x-7'
                                            : 'translate-x-1'}"
                                    ></span>
                                </button>
                            {/if}
                        </div>

                        <div
                            class="row-span-1 col-span-12 mb-2 h-56 bg-[#0a0a0a] rounded-xl p-4 border border-[#2a2a2a] relative {getFeature(
                                'Equalizer',
                            )?.value?.Toggle
                                ? ''
                                : 'opacity-50 pointer-events-none transition-opacity'} mx-auto w-[90%] md:w-full"
                        >
                            <div class="w-full h-full relative">
                                <canvas
                                    bind:this={eqCanvas}
                                    class="absolute top-0 left-0 w-full h-full cursor-pointer"
                                    id="eqCanvas"
                                    on:mousedown={handleCanvasDown}
                                    on:mousemove={handleCanvasMove}
                                    on:mouseup={handleCanvasUp}
                                    on:mouseleave={handleCanvasUp}
                                    on:touchstart|preventDefault={(e) =>
                                        handleCanvasDown({
                                            clientX: e.touches[0].clientX,
                                            clientY: e.touches[0].clientY,
                                        })}
                                    on:touchmove|preventDefault={(e) =>
                                        handleCanvasMove({
                                            clientX: e.touches[0].clientX,
                                            clientY: e.touches[0].clientY,
                                        })}
                                    on:touchend={handleCanvasUp}
                                ></canvas>
                            </div>
                        </div>

                        <div
                            class="grid grid-cols-1 md:grid-cols-11 lg:grid-cols-11 xl:grid-cols-11 gap-x-1 gap-y-4 justify-items-center {getFeature(
                                'Equalizer',
                            )?.value?.Toggle
                                ? ''
                                : 'opacity-50 pointer-events-none'}"
                        >
                            <!-- Pre-Amp -->
                            <div
                                class="col-span-1 flex flex-col items-center h-[22rem] space-y-2 group"
                            >
                                <span
                                    class="text-[10px] text-gray-500 font-mono tracking-wider group-hover:text-gray-300"
                                    >PRE</span
                                >
                                <div
                                    class="h-[12rem] flex justify-center items-center py-2 bg-[#0a0a0a] rounded-xl w-12 border border-[#1a1a1a] shadow-inner relative group-hover:bg-[#111] transition-colors"
                                >
                                    <!-- Ticks behind slider -->
                                    <div
                                        class="absolute h-[11rem] w-full flex flex-col justify-between pointer-events-none py-1 z-0"
                                    >
                                        <div
                                            class="flex items-center justify-between px-1 opacity-50"
                                        >
                                            <span
                                                class="text-[8px] text-gray-400"
                                                >+12</span
                                            >
                                            <div
                                                class="w-1.5 border-t border-gray-600"
                                            ></div>
                                        </div>
                                        <div
                                            class="flex items-center justify-end px-1 opacity-20"
                                        >
                                            <div
                                                class="w-1.5 border-t border-gray-600"
                                            ></div>
                                        </div>
                                        <div
                                            class="flex items-center justify-between px-1 opacity-50"
                                        >
                                            <span
                                                class="text-[8px] text-gray-500"
                                            >
                                                0
                                            </span>
                                            <div
                                                class="w-1.5 border-t border-red-900/50"
                                            ></div>
                                        </div>
                                        <div
                                            class="flex items-center justify-end px-1 opacity-20"
                                        >
                                            <div
                                                class="w-1.5 border-t border-gray-600"
                                            ></div>
                                        </div>
                                        <div
                                            class="flex items-center justify-between px-1 opacity-50"
                                        >
                                            <span
                                                class="text-[8px] text-gray-400"
                                                >-12</span
                                            >
                                            <div
                                                class="w-1.5 border-t border-gray-600"
                                            ></div>
                                        </div>
                                    </div>
                                    {#if getFeature("EQ Pre-Amp")}
                                        {@const preAmpName = "EQ Pre-Amp"}
                                        {@const preAmpVal =
                                            eqLocalValues[preAmpName] !==
                                            undefined
                                                ? eqLocalValues[preAmpName]
                                                : getFeature(preAmpName)?.value
                                                      ?.Slider || 0}
                                        <input
                                            type="range"
                                            min="-12"
                                            max="12"
                                            step="0.1"
                                            value={preAmpVal}
                                            on:mousedown={() =>
                                                (isDraggingSlider = true)}
                                            on:mouseup={() =>
                                                (isDraggingSlider = false)}
                                            on:touchstart={() =>
                                                (isDraggingSlider = true)}
                                            on:touchend={() =>
                                                (isDraggingSlider = false)}
                                            on:input={(e) => {
                                                const v = parseFloat(
                                                    e.currentTarget.value,
                                                );
                                                eqLocalValues[preAmpName] = v;
                                                device.updateFeature(
                                                    preAmpName,
                                                    v,
                                                );
                                            }}
                                            class="absolute w-[11rem] h-2 bg-transparent z-10 appearance-none cursor-pointer accent-red-600 hover:accent-red-500 -rotate-90 origin-center"
                                        />
                                    {:else}
                                        <div
                                            class="w-1 h-full bg-gray-800"
                                        ></div>
                                    {/if}
                                </div>
                                <span
                                    class="text-[11px] text-red-500 font-bold whitespace-nowrap bg-red-500/10 px-2 py-0.5 rounded"
                                    >{eqLocalValues["EQ Pre-Amp"] > 0
                                        ? "+"
                                        : ""}{(eqLocalValues["EQ Pre-Amp"] !==
                                    undefined
                                        ? eqLocalValues["EQ Pre-Amp"]
                                        : getFeature("EQ Pre-Amp")?.value
                                              ?.Slider || 0
                                    ).toFixed(1)} dB</span
                                >
                            </div>

                            <!-- Bands -->
                            {#each ["31Hz", "62Hz", "125Hz", "250Hz", "500Hz", "1kHz", "2kHz", "4kHz", "8kHz", "16kHz"] as band}
                                {@const featureName = "EQ " + band}
                                {@const bandFeat = getFeature(featureName)}
                                {@const bandVal =
                                    eqLocalValues[featureName] !== undefined
                                        ? eqLocalValues[featureName]
                                        : bandFeat?.value?.Slider || 0}
                                <div
                                    class="col-span-1 flex flex-col items-center h-[22rem] space-y-2 group"
                                >
                                    <span
                                        class="text-[10px] text-gray-500 font-mono tracking-wider group-hover:text-gray-300"
                                        >{band.toUpperCase()}</span
                                    >
                                    <div
                                        class="h-[12rem] flex justify-center items-center py-2 bg-[#0a0a0a] rounded-xl w-12 border border-[#1a1a1a] shadow-inner relative group-hover:bg-[#111] transition-colors"
                                    >
                                        <!-- Ticks behind slider -->
                                        <div
                                            class="absolute h-[11rem] w-full flex flex-col justify-between pointer-events-none py-1 z-0"
                                        >
                                            <div
                                                class="flex items-center justify-between px-1 opacity-50"
                                            >
                                                <span
                                                    class="text-[8px] text-gray-400 opacity-0"
                                                    >+12</span
                                                >
                                                <div
                                                    class="w-1.5 border-t border-gray-600"
                                                ></div>
                                            </div>
                                            <div
                                                class="flex items-center justify-end px-1 opacity-20"
                                            >
                                                <div
                                                    class="w-1.5 border-t border-gray-600"
                                                ></div>
                                            </div>
                                            <div
                                                class="flex items-center justify-between px-1 opacity-50"
                                            >
                                                <span
                                                    class="text-[8px] text-gray-500 opacity-0"
                                                >
                                                    0
                                                </span>
                                                <div
                                                    class="w-1.5 border-t border-red-900/50"
                                                ></div>
                                            </div>
                                            <div
                                                class="flex items-center justify-end px-1 opacity-20"
                                            >
                                                <div
                                                    class="w-1.5 border-t border-gray-600"
                                                ></div>
                                            </div>
                                            <div
                                                class="flex items-center justify-between px-1 opacity-50"
                                            >
                                                <span
                                                    class="text-[8px] text-gray-400 opacity-0"
                                                    >-12</span
                                                >
                                                <div
                                                    class="w-1.5 border-t border-gray-600"
                                                ></div>
                                            </div>
                                        </div>

                                        {#if bandFeat}
                                            <input
                                                type="range"
                                                min="-12"
                                                max="12"
                                                step="0.1"
                                                value={bandVal}
                                                on:mousedown={() =>
                                                    (isDraggingSlider = true)}
                                                on:mouseup={() =>
                                                    (isDraggingSlider = false)}
                                                on:touchstart={() =>
                                                    (isDraggingSlider = true)}
                                                on:touchend={() =>
                                                    (isDraggingSlider = false)}
                                                on:input={(e) => {
                                                    const v = parseFloat(
                                                        e.currentTarget.value,
                                                    );
                                                    eqLocalValues[featureName] =
                                                        v;
                                                    device.updateFeature(
                                                        featureName,
                                                        v,
                                                    );
                                                }}
                                                class="absolute w-[11rem] h-2 bg-transparent z-10 appearance-none cursor-pointer accent-red-600 hover:accent-red-400 -rotate-90 origin-center"
                                            />
                                        {/if}
                                    </div>
                                    <span
                                        class="text-[11px] text-gray-400 font-bold whitespace-nowrap bg-[#1a1a1a] px-2 py-0.5 rounded group-hover:text-white group-hover:bg-[#222] transition-colors"
                                        >{bandVal > 0
                                            ? "+"
                                            : ""}{bandVal.toFixed(1)} dB</span
                                    >
                                </div>
                            {/each}
                        </div>
                    </div>
                {:else if activeTab === "mixer"}
                    <div class="space-y-6">
                        <!-- ══════════════════════════════════════════════════ -->
                        <!-- MASTER OUTPUT (L / R) - Dedicated Premium Card    -->
                        <!-- ══════════════════════════════════════════════════ -->
                        {#if $device.mixer && $device.mixer["Speaker"] && $device.mixer["Speaker"]["playback_vol"] !== undefined}
                            {@const spkData = $device.mixer["Speaker"]}
                            {@const spkVol = spkData.playback_vol || 0}
                            {@const spkL =
                                spkData.playback_vol_l !== undefined
                                    ? spkData.playback_vol_l
                                    : spkVol}
                            {@const spkR =
                                spkData.playback_vol_r !== undefined
                                    ? spkData.playback_vol_r
                                    : spkVol}
                            {@const balanceRaw =
                                spkL + spkR > 0
                                    ? (spkR - spkL) / Math.max(spkL, spkR, 0.01)
                                    : 0}
                            <div
                                class="bg-gradient-to-br from-[#0d1117] via-[#111827] to-[#0f172a] border border-blue-500/20 rounded-2xl p-8 relative overflow-hidden shadow-xl shadow-blue-900/10"
                            >
                                <div
                                    class="absolute inset-0 bg-[url('/noise.png')] opacity-[0.03] pointer-events-none"
                                ></div>
                                <div
                                    class="absolute top-0 left-0 w-full h-1 bg-gradient-to-r from-blue-600 via-cyan-500 to-blue-600 opacity-60"
                                ></div>

                                <!-- Header -->
                                <div
                                    class="flex items-center justify-between mb-8 relative z-10"
                                >
                                    <div class="flex items-center space-x-4">
                                        <div
                                            class="p-3 bg-blue-500/10 rounded-xl text-blue-400 shadow-[0_0_20px_rgba(59,130,246,0.15)]"
                                        >
                                            <Headphones size={28} />
                                        </div>
                                        <div>
                                            <h3
                                                class="font-bold text-white text-xl tracking-tight"
                                            >
                                                Master Output
                                            </h3>
                                            <p
                                                class="text-xs text-gray-500 mt-0.5"
                                            >
                                                Speaker Stereo Channel Control
                                            </p>
                                        </div>
                                    </div>
                                    <div class="text-right">
                                        <div
                                            class="text-3xl font-black text-blue-400 font-mono tabular-nums"
                                        >
                                            {(spkVol * 100).toFixed(0)}<span
                                                class="text-lg text-blue-500/60"
                                                >%</span
                                            >
                                        </div>
                                        <div
                                            class="text-[10px] text-gray-600 uppercase tracking-widest"
                                        >
                                            avg volume
                                        </div>
                                    </div>
                                </div>

                                <!-- L / R Channel Controls -->
                                <div
                                    class="grid grid-cols-1 md:grid-cols-2 gap-6 relative z-10"
                                >
                                    <!-- LEFT CHANNEL -->
                                    <div
                                        class="bg-[#0a0e14] border border-blue-500/10 rounded-xl p-5 space-y-4 hover:border-blue-500/30 transition-all duration-300"
                                    >
                                        <div
                                            class="flex items-center justify-between"
                                        >
                                            <div
                                                class="flex items-center space-x-3"
                                            >
                                                <div
                                                    class="w-8 h-8 rounded-full bg-blue-600/20 flex items-center justify-center"
                                                >
                                                    <span
                                                        class="text-blue-400 font-black text-sm"
                                                        >L</span
                                                    >
                                                </div>
                                                <div>
                                                    <div
                                                        class="text-sm font-bold text-white"
                                                    >
                                                        Left Channel
                                                    </div>
                                                    <div
                                                        class="text-[10px] text-gray-600 uppercase tracking-wider"
                                                    >
                                                        Front Left
                                                    </div>
                                                </div>
                                            </div>
                                            <div class="text-right">
                                                <span
                                                    class="text-2xl font-black text-blue-300 font-mono tabular-nums"
                                                    >{(spkL * 100).toFixed(
                                                        0,
                                                    )}</span
                                                >
                                                <span
                                                    class="text-xs text-blue-500/50 font-bold"
                                                    >%</span
                                                >
                                            </div>
                                        </div>
                                        <!-- Volume Bar Visual -->
                                        <div
                                            class="relative h-3 bg-[#111827] rounded-full overflow-hidden border border-blue-900/30"
                                        >
                                            <div
                                                class="absolute inset-y-0 left-0 rounded-full transition-all duration-150"
                                                style="width: {spkL *
                                                    100}%; background: linear-gradient(90deg, #1d4ed8, #3b82f6, #60a5fa);"
                                            ></div>
                                        </div>
                                        <!-- Slider -->
                                        <input
                                            type="range"
                                            min="0"
                                            max="1"
                                            step="0.01"
                                            value={spkL}
                                            on:input={(e) =>
                                                device.updateMixer("Speaker", {
                                                    playback_vol_l: parseFloat(
                                                        e.currentTarget.value,
                                                    ),
                                                    playback_vol_r: spkR,
                                                })}
                                            class="w-full h-2 bg-gray-800 rounded-lg appearance-none cursor-pointer accent-blue-500 hover:accent-blue-400"
                                        />
                                    </div>

                                    <!-- RIGHT CHANNEL -->
                                    <div
                                        class="bg-[#0a0e14] border border-cyan-500/10 rounded-xl p-5 space-y-4 hover:border-cyan-500/30 transition-all duration-300"
                                    >
                                        <div
                                            class="flex items-center justify-between"
                                        >
                                            <div
                                                class="flex items-center space-x-3"
                                            >
                                                <div
                                                    class="w-8 h-8 rounded-full bg-cyan-600/20 flex items-center justify-center"
                                                >
                                                    <span
                                                        class="text-cyan-400 font-black text-sm"
                                                        >R</span
                                                    >
                                                </div>
                                                <div>
                                                    <div
                                                        class="text-sm font-bold text-white"
                                                    >
                                                        Right Channel
                                                    </div>
                                                    <div
                                                        class="text-[10px] text-gray-600 uppercase tracking-wider"
                                                    >
                                                        Front Right
                                                    </div>
                                                </div>
                                            </div>
                                            <div class="text-right">
                                                <span
                                                    class="text-2xl font-black text-cyan-300 font-mono tabular-nums"
                                                    >{(spkR * 100).toFixed(
                                                        0,
                                                    )}</span
                                                >
                                                <span
                                                    class="text-xs text-cyan-500/50 font-bold"
                                                    >%</span
                                                >
                                            </div>
                                        </div>
                                        <!-- Volume Bar Visual -->
                                        <div
                                            class="relative h-3 bg-[#111827] rounded-full overflow-hidden border border-cyan-900/30"
                                        >
                                            <div
                                                class="absolute inset-y-0 left-0 rounded-full transition-all duration-150"
                                                style="width: {spkR *
                                                    100}%; background: linear-gradient(90deg, #0e7490, #06b6d4, #67e8f9);"
                                            ></div>
                                        </div>
                                        <!-- Slider -->
                                        <input
                                            type="range"
                                            min="0"
                                            max="1"
                                            step="0.01"
                                            value={spkR}
                                            on:input={(e) =>
                                                device.updateMixer("Speaker", {
                                                    playback_vol_l: spkL,
                                                    playback_vol_r: parseFloat(
                                                        e.currentTarget.value,
                                                    ),
                                                })}
                                            class="w-full h-2 bg-gray-800 rounded-lg appearance-none cursor-pointer accent-cyan-500 hover:accent-cyan-400"
                                        />
                                    </div>
                                </div>

                                <!-- Balance Indicator -->
                                <div class="mt-6 relative z-10">
                                    <div
                                        class="flex items-center justify-between text-[10px] text-gray-600 uppercase tracking-widest mb-2"
                                    >
                                        <span>Left</span>
                                        <span class="font-bold text-gray-400">
                                            Balance: {Math.abs(balanceRaw) <
                                            0.02
                                                ? "Center"
                                                : balanceRaw < 0
                                                  ? `L ${(Math.abs(balanceRaw) * 100).toFixed(0)}%`
                                                  : `R ${(balanceRaw * 100).toFixed(0)}%`}
                                        </span>
                                        <span>Right</span>
                                    </div>
                                    <div
                                        class="relative h-2 bg-[#111827] rounded-full overflow-hidden border border-gray-800"
                                    >
                                        <div
                                            class="absolute inset-y-0 left-1/2 w-0.5 bg-gray-600 z-10"
                                        ></div>
                                        {#if balanceRaw < 0}
                                            <div
                                                class="absolute inset-y-0 rounded-full transition-all duration-150"
                                                style="right: 50%; width: {Math.abs(
                                                    balanceRaw,
                                                ) *
                                                    50}%; background: linear-gradient(90deg, #3b82f6, #60a5fa);"
                                            ></div>
                                        {:else}
                                            <div
                                                class="absolute inset-y-0 rounded-full transition-all duration-150"
                                                style="left: 50%; width: {balanceRaw *
                                                    50}%; background: linear-gradient(90deg, #06b6d4, #67e8f9);"
                                            ></div>
                                        {/if}
                                    </div>
                                </div>
                            </div>
                        {/if}

                        <!-- ══════════════════════════════════════════════════ -->
                        <!-- MICROPHONE INPUT (L / R) - Dedicated Premium Card -->
                        <!-- ══════════════════════════════════════════════════ -->
                        {#if $device.mixer && $device.mixer["External Mic"] && $device.mixer["External Mic"]["capture_vol"] !== undefined}
                            {@const micData = $device.mixer["External Mic"]}
                            {@const micVol = micData.capture_vol || 0}
                            {@const micL =
                                micData.capture_vol_l !== undefined
                                    ? micData.capture_vol_l
                                    : micVol}
                            {@const micR =
                                micData.capture_vol_r !== undefined
                                    ? micData.capture_vol_r
                                    : micVol}
                            {@const micBalanceRaw =
                                micL + micR > 0
                                    ? (micR - micL) / Math.max(micL, micR, 0.01)
                                    : 0}
                            <div
                                class="bg-gradient-to-br from-[#170d0d] via-[#1a1111] to-[#1c0f14] border border-red-500/20 rounded-2xl p-8 relative overflow-hidden shadow-xl shadow-red-900/10"
                            >
                                <div
                                    class="absolute inset-0 bg-[url('/noise.png')] opacity-[0.03] pointer-events-none"
                                ></div>
                                <div
                                    class="absolute top-0 left-0 w-full h-1 bg-gradient-to-r from-red-600 via-rose-500 to-red-600 opacity-60"
                                ></div>

                                <!-- Header -->
                                <div
                                    class="flex items-center justify-between mb-8 relative z-10"
                                >
                                    <div class="flex items-center space-x-4">
                                        <div
                                            class="p-3 bg-red-500/10 rounded-xl text-red-400 shadow-[0_0_20px_rgba(220,38,38,0.15)]"
                                        >
                                            <Mic size={28} />
                                        </div>
                                        <div>
                                            <h3
                                                class="font-bold text-white text-xl tracking-tight"
                                            >
                                                Microphone Input
                                            </h3>
                                            <p
                                                class="text-xs text-gray-500 mt-0.5"
                                            >
                                                External Mic Stereo Channel
                                                Control
                                            </p>
                                        </div>
                                    </div>
                                    <div class="text-right">
                                        <div
                                            class="text-3xl font-black text-red-400 font-mono tabular-nums"
                                        >
                                            {(micVol * 100).toFixed(0)}<span
                                                class="text-lg text-red-500/60"
                                                >%</span
                                            >
                                        </div>
                                        <div
                                            class="text-[10px] text-gray-600 uppercase tracking-widest"
                                        >
                                            avg level
                                        </div>
                                    </div>
                                </div>

                                <!-- L / R Channel Controls -->
                                <div
                                    class="grid grid-cols-1 md:grid-cols-2 gap-6 relative z-10"
                                >
                                    <!-- LEFT CHANNEL -->
                                    <div
                                        class="bg-[#120a0a] border border-red-500/10 rounded-xl p-5 space-y-4 hover:border-red-500/30 transition-all duration-300"
                                    >
                                        <div
                                            class="flex items-center justify-between"
                                        >
                                            <div
                                                class="flex items-center space-x-3"
                                            >
                                                <div
                                                    class="w-8 h-8 rounded-full bg-red-600/20 flex items-center justify-center"
                                                >
                                                    <span
                                                        class="text-red-400 font-black text-sm"
                                                        >L</span
                                                    >
                                                </div>
                                                <div>
                                                    <div
                                                        class="text-sm font-bold text-white"
                                                    >
                                                        Left Channel
                                                    </div>
                                                    <div
                                                        class="text-[10px] text-gray-600 uppercase tracking-wider"
                                                    >
                                                        Capture Left
                                                    </div>
                                                </div>
                                            </div>
                                            <div class="text-right">
                                                <span
                                                    class="text-2xl font-black text-red-300 font-mono tabular-nums"
                                                    >{(micL * 100).toFixed(
                                                        0,
                                                    )}</span
                                                >
                                                <span
                                                    class="text-xs text-red-500/50 font-bold"
                                                    >%</span
                                                >
                                            </div>
                                        </div>
                                        <!-- Volume Bar Visual -->
                                        <div
                                            class="relative h-3 bg-[#1a0f0f] rounded-full overflow-hidden border border-red-900/30"
                                        >
                                            <div
                                                class="absolute inset-y-0 left-0 rounded-full transition-all duration-150"
                                                style="width: {micL *
                                                    100}%; background: linear-gradient(90deg, #991b1b, #dc2626, #f87171);"
                                            ></div>
                                        </div>
                                        <!-- Slider -->
                                        <input
                                            type="range"
                                            min="0"
                                            max="1"
                                            step="0.01"
                                            value={micL}
                                            on:input={(e) =>
                                                device.updateMixer(
                                                    "External Mic",
                                                    {
                                                        capture_vol_l:
                                                            parseFloat(
                                                                e.currentTarget
                                                                    .value,
                                                            ),
                                                        capture_vol_r: micR,
                                                    },
                                                )}
                                            class="w-full h-2 bg-gray-800 rounded-lg appearance-none cursor-pointer accent-red-500 hover:accent-red-400"
                                        />
                                    </div>

                                    <!-- RIGHT CHANNEL -->
                                    <div
                                        class="bg-[#120a0a] border border-rose-500/10 rounded-xl p-5 space-y-4 hover:border-rose-500/30 transition-all duration-300"
                                    >
                                        <div
                                            class="flex items-center justify-between"
                                        >
                                            <div
                                                class="flex items-center space-x-3"
                                            >
                                                <div
                                                    class="w-8 h-8 rounded-full bg-rose-600/20 flex items-center justify-center"
                                                >
                                                    <span
                                                        class="text-rose-400 font-black text-sm"
                                                        >R</span
                                                    >
                                                </div>
                                                <div>
                                                    <div
                                                        class="text-sm font-bold text-white"
                                                    >
                                                        Right Channel
                                                    </div>
                                                    <div
                                                        class="text-[10px] text-gray-600 uppercase tracking-wider"
                                                    >
                                                        Capture Right
                                                    </div>
                                                </div>
                                            </div>
                                            <div class="text-right">
                                                <span
                                                    class="text-2xl font-black text-rose-300 font-mono tabular-nums"
                                                    >{(micR * 100).toFixed(
                                                        0,
                                                    )}</span
                                                >
                                                <span
                                                    class="text-xs text-rose-500/50 font-bold"
                                                    >%</span
                                                >
                                            </div>
                                        </div>
                                        <!-- Volume Bar Visual -->
                                        <div
                                            class="relative h-3 bg-[#1a0f0f] rounded-full overflow-hidden border border-rose-900/30"
                                        >
                                            <div
                                                class="absolute inset-y-0 left-0 rounded-full transition-all duration-150"
                                                style="width: {micR *
                                                    100}%; background: linear-gradient(90deg, #9f1239, #e11d48, #fb7185);"
                                            ></div>
                                        </div>
                                        <!-- Slider -->
                                        <input
                                            type="range"
                                            min="0"
                                            max="1"
                                            step="0.01"
                                            value={micR}
                                            on:input={(e) =>
                                                device.updateMixer(
                                                    "External Mic",
                                                    {
                                                        capture_vol_l: micL,
                                                        capture_vol_r:
                                                            parseFloat(
                                                                e.currentTarget
                                                                    .value,
                                                            ),
                                                    },
                                                )}
                                            class="w-full h-2 bg-gray-800 rounded-lg appearance-none cursor-pointer accent-rose-500 hover:accent-rose-400"
                                        />
                                    </div>
                                </div>

                                <!-- Balance Indicator -->
                                <div class="mt-6 relative z-10">
                                    <div
                                        class="flex items-center justify-between text-[10px] text-gray-600 uppercase tracking-widest mb-2"
                                    >
                                        <span>Left</span>
                                        <span class="font-bold text-gray-400">
                                            Balance: {Math.abs(micBalanceRaw) <
                                            0.02
                                                ? "Center"
                                                : micBalanceRaw < 0
                                                  ? `L ${(Math.abs(micBalanceRaw) * 100).toFixed(0)}%`
                                                  : `R ${(micBalanceRaw * 100).toFixed(0)}%`}
                                        </span>
                                        <span>Right</span>
                                    </div>
                                    <div
                                        class="relative h-2 bg-[#1a0f0f] rounded-full overflow-hidden border border-gray-800"
                                    >
                                        <div
                                            class="absolute inset-y-0 left-1/2 w-0.5 bg-gray-600 z-10"
                                        ></div>
                                        {#if micBalanceRaw < 0}
                                            <div
                                                class="absolute inset-y-0 rounded-full transition-all duration-150"
                                                style="right: 50%; width: {Math.abs(
                                                    micBalanceRaw,
                                                ) *
                                                    50}%; background: linear-gradient(90deg, #dc2626, #f87171);"
                                            ></div>
                                        {:else}
                                            <div
                                                class="absolute inset-y-0 rounded-full transition-all duration-150"
                                                style="left: 50%; width: {micBalanceRaw *
                                                    50}%; background: linear-gradient(90deg, #e11d48, #fb7185);"
                                            ></div>
                                        {/if}
                                    </div>
                                </div>
                            </div>
                        {/if}

                        <!-- ══════════════════════════════════════════════════ -->
                        <!-- GENERAL MIXER CHANNELS (Playback + Recording)     -->
                        <!-- ══════════════════════════════════════════════════ -->
                        <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
                            <!-- Playback Controls -->
                            <div
                                class="bg-[#1a1a1a] border border-[#2a2a2a] rounded-2xl p-8 relative overflow-hidden"
                            >
                                <div
                                    class="absolute inset-0 bg-[url('/noise.png')] opacity-[0.02] pointer-events-none"
                                ></div>
                                <div
                                    class="flex items-center space-x-4 mb-8 relative z-10"
                                >
                                    <div
                                        class="p-3 bg-blue-600/10 rounded-lg text-blue-500"
                                    >
                                        <Speaker size={24} />
                                    </div>
                                    <div>
                                        <h3
                                            class="font-bold text-white text-lg"
                                        >
                                            Playback Channels
                                        </h3>
                                        <p class="text-sm text-gray-500">
                                            Sidetone & routing levels
                                        </p>
                                    </div>
                                </div>
                                <div class="space-y-6 relative z-10">
                                    {#each [{ name: "Speaker", label: "Master Output", key: "playback_vol" }, { name: "Line In", label: "Line In (Sidetone)", key: "playback_vol" }, { name: "External Mic", label: "Mic (Sidetone)", key: "playback_vol" }, { name: "S/PDIF In", label: "S/PDIF (Sidetone)", key: "playback_vol" }] as item}
                                        {#if $device.mixer && $device.mixer[item.name] && $device.mixer[item.name][item.key] !== undefined}
                                            {@const val =
                                                $device.mixer[item.name][
                                                    item.key
                                                ] || 0}
                                            <div>
                                                <div
                                                    class="flex justify-between text-sm mb-2"
                                                >
                                                    <span
                                                        class="text-gray-300 font-medium"
                                                        >{item.label}</span
                                                    >
                                                    <span
                                                        class="text-gray-500 font-mono"
                                                        >{(val * 100).toFixed(
                                                            0,
                                                        )}%</span
                                                    >
                                                </div>
                                                <input
                                                    type="range"
                                                    min="0"
                                                    max="1"
                                                    step="0.01"
                                                    value={val}
                                                    on:input={(e) =>
                                                        device.updateMixer(
                                                            item.name,
                                                            {
                                                                [item.key]:
                                                                    parseFloat(
                                                                        e
                                                                            .currentTarget
                                                                            .value,
                                                                    ),
                                                            },
                                                        )}
                                                    class="w-full h-2 bg-gray-700 rounded-lg appearance-none cursor-pointer accent-blue-500 hover:accent-blue-400"
                                                />
                                            </div>
                                        {/if}
                                    {/each}
                                </div>
                            </div>

                            <!-- Recording Controls -->
                            <div
                                class="bg-[#1a1a1a] border border-[#2a2a2a] rounded-2xl p-8 relative overflow-hidden"
                            >
                                <div
                                    class="absolute inset-0 bg-[url('/noise.png')] opacity-[0.02] pointer-events-none"
                                ></div>
                                <div
                                    class="flex items-center space-x-4 mb-8 relative z-10"
                                >
                                    <div
                                        class="p-3 bg-red-600/10 rounded-lg text-red-500"
                                    >
                                        <Mic size={24} />
                                    </div>
                                    <div>
                                        <h3
                                            class="font-bold text-white text-lg"
                                        >
                                            Recording Channels
                                        </h3>
                                        <p class="text-sm text-gray-500">
                                            Input capture levels
                                        </p>
                                    </div>
                                </div>
                                <div class="space-y-6 relative z-10">
                                    {#each [{ name: "External Mic", label: "Microphone Input", key: "capture_vol" }, { name: "Line In", label: "Line In Input", key: "capture_vol" }, { name: "S/PDIF In", label: "S/PDIF Input", key: "capture_vol" }, { name: "What U Hear", label: "What U Hear (Stereo Mix)", key: "capture_vol" }] as item}
                                        {#if $device.mixer && $device.mixer[item.name] && $device.mixer[item.name][item.key] !== undefined}
                                            {@const val =
                                                $device.mixer[item.name][
                                                    item.key
                                                ] || 0}
                                            <div>
                                                <div
                                                    class="flex justify-between text-sm mb-2"
                                                >
                                                    <span
                                                        class="text-gray-300 font-medium"
                                                        >{item.label}</span
                                                    >
                                                    <span
                                                        class="text-gray-500 font-mono"
                                                        >{(val * 100).toFixed(
                                                            0,
                                                        )}%</span
                                                    >
                                                </div>
                                                <input
                                                    type="range"
                                                    min="0"
                                                    max="1"
                                                    step="0.01"
                                                    value={val}
                                                    on:input={(e) =>
                                                        device.updateMixer(
                                                            item.name,
                                                            {
                                                                [item.key]:
                                                                    parseFloat(
                                                                        e
                                                                            .currentTarget
                                                                            .value,
                                                                    ),
                                                            },
                                                        )}
                                                    class="w-full h-2 bg-gray-700 rounded-lg appearance-none cursor-pointer accent-red-600 hover:accent-red-500"
                                                />
                                            </div>
                                        {/if}
                                    {/each}
                                </div>
                            </div>
                        </div>

                        <!-- 7.1 Surround Spatial Audio Control -->
                        <div
                            class="col-span-1 lg:col-span-2 bg-gradient-to-r from-[#1a1a1a] to-[#221010] border border-red-500/20 rounded-2xl p-8 relative overflow-hidden shadow-lg shadow-red-900/10 mb-4"
                        >
                            <!-- Background elements -->
                            <div
                                class="absolute inset-0 bg-[url('/noise.png')] opacity-[0.03] pointer-events-none"
                            ></div>

                            <div
                                class="flex flex-col lg:flex-row items-start lg:items-center justify-between relative z-10 gap-6"
                            >
                                <div class="flex items-center space-x-5">
                                    <div
                                        class="p-4 bg-red-600/10 rounded-xl text-red-500 shadow-[0_0_15px_rgba(220,38,38,0.2)]"
                                    >
                                        <Monitor size={32} />
                                    </div>
                                    <div>
                                        <h3
                                            class="font-bold text-white text-xl flex items-center gap-3"
                                        >
                                            7.1 Surround Sound
                                            <span
                                                class="text-[10px] uppercase font-bold bg-red-600/20 text-red-400 px-2 py-1 rounded border border-red-500/30"
                                                >Virtualization</span
                                            >
                                        </h3>
                                        <p
                                            class="text-sm text-gray-400 mt-1 max-w-md"
                                        >
                                            Enable precise 3D spatial audio and
                                            acoustic immersion. Control the
                                            surround virtualization distance
                                            spread accurately.
                                        </p>
                                    </div>
                                </div>

                                {#if getFeature("Surround")}
                                    {@const surroundFeat =
                                        getFeature("Surround")}
                                    {@const surroundSlider =
                                        getSlider("Surround")}
                                    <div
                                        class="flex flex-col lg:items-end w-full lg:w-1/2 space-y-4"
                                    >
                                        <!-- Toggle switch -->
                                        <div
                                            class="flex items-center space-x-4"
                                        >
                                            <span
                                                class="text-gray-400 font-bold uppercase text-xs tracking-wider"
                                                >{surroundFeat.value.Toggle
                                                    ? "Activated"
                                                    : "Disabled"}</span
                                            >
                                            <button
                                                class="relative inline-flex h-8 w-14 items-center rounded-full transition-colors focus:outline-none shadow-lg {surroundFeat
                                                    .value.Toggle
                                                    ? 'bg-red-600 shadow-[0_0_12px_rgba(220,38,38,0.6)]'
                                                    : 'bg-gray-800 border border-[#333]'}"
                                                on:click={() =>
                                                    toggleFeature("Surround")}
                                            >
                                                <span class="sr-only"
                                                    >Toggle 7.1 Surround</span
                                                >
                                                <span
                                                    class="inline-block h-6 w-6 transform rounded-full bg-white transition-transform {surroundFeat
                                                        .value.Toggle
                                                        ? 'translate-x-[26px]'
                                                        : 'translate-x-1'}"
                                                ></span>
                                            </button>
                                        </div>

                                        <!-- Spread Slider -->
                                        <div
                                            class="w-full bg-[#111] p-4 rounded-xl border border-[#333]"
                                        >
                                            <div
                                                class="flex justify-between text-xs mb-2"
                                            >
                                                <span
                                                    class="text-gray-300 font-bold tracking-wide"
                                                    >Surround Depth Spread (7.1
                                                    Array)</span
                                                >
                                                <span
                                                    class="text-red-400 font-mono font-bold"
                                                    >{surroundSlider.toFixed(
                                                        0,
                                                    )}</span
                                                >
                                            </div>
                                            <input
                                                type="range"
                                                min="0"
                                                max="100"
                                                step="1"
                                                value={surroundSlider}
                                                on:change={(e) =>
                                                    device.updateFeature(
                                                        "Surround Slider",
                                                        parseFloat(
                                                            e.currentTarget
                                                                .value,
                                                        ),
                                                    )}
                                                class="w-full h-2 bg-gray-800 rounded-lg appearance-none cursor-pointer accent-red-500 hover:accent-red-400 transition-all {surroundFeat
                                                    .value.Toggle
                                                    ? 'opacity-100'
                                                    : 'opacity-40 grayscale pointer-events-none'}"
                                                disabled={!surroundFeat.value
                                                    .Toggle}
                                            />
                                        </div>
                                    </div>
                                {/if}
                            </div>
                        </div>
                    </div>
                {/if}
            </div>
        </main>
    </div>
    <!-- End Top Layout Area -->

    <!-- Global Full-Width Footer -->
    {#if $device && !$device.loading}
        <footer
            class="shrink-0 border-t border-[#2a2a2a] bg-[#141414] flex flex-col md:flex-row shadow-[0_-10px_30px_rgba(0,0,0,0.5)] relative z-50 w-full"
        >
            <!-- Device Status Box (matches sidebar width exactly: w-64) -->
            <div
                class="hidden md:flex w-64 shrink-0 p-4 border-r border-[#2a2a2a] items-center bg-[#1a1a1a]"
            >
                <div
                    class="w-full bg-[#252525] rounded-xl p-3 flex items-center justify-between"
                >
                    <div class="flex flex-col">
                        <div class="flex items-center space-x-3">
                            <div
                                class="w-2 h-2 rounded-full {!$device.error &&
                                !$device.loading
                                    ? 'animate-pulse bg-green-500 shadow-[0_0_8px_rgba(34,197,94,0.6)]'
                                    : 'bg-red-500 shadow-[0_0_8px_rgba(220,38,38,0.6)]'}"
                            ></div>
                            <span
                                class="text-xs font-bold uppercase tracking-wider {!$device.error &&
                                !$device.loading
                                    ? 'text-green-400'
                                    : 'text-red-400'}"
                                >Device {!$device.error && !$device.loading
                                    ? "Connected"
                                    : "Offline"}</span
                            >
                        </div>
                        {#if $device.error}
                            <span class="text-[10px] text-red-500/70 mt-1 ml-5"
                                >{$device.error}</span
                            >
                        {/if}
                    </div>
                </div>
            </div>

            <!-- Global Volume Controls -->
            <div
                class="flex-1 p-4 px-8 flex flex-col lg:flex-row items-center justify-center gap-8 bg-[#141414]"
            >
                <!-- Global Output Volume -->
                <div
                    class="flex-1 flex items-center space-x-4 w-full md:max-w-md"
                >
                    <button
                        class="p-2 rounded-lg transition-all duration-200 focus:outline-none {$device.mixer &&
                        $device.mixer['Speaker'] &&
                        $device.mixer['Speaker']['playback_mute']
                            ? 'bg-red-600/20 text-red-500 shadow-[0_0_10px_rgba(220,38,38,0.3)]'
                            : 'bg-blue-600/10 text-blue-500 hover:bg-blue-600/20'}"
                        title={$device.mixer &&
                        $device.mixer["Speaker"] &&
                        $device.mixer["Speaker"]["playback_mute"]
                            ? "Unmute Speakers"
                            : "Mute Speakers"}
                        on:click={() =>
                            device.updateMixer("Speaker", {
                                playback_mute: !(
                                    $device.mixer &&
                                    $device.mixer["Speaker"] &&
                                    $device.mixer["Speaker"]["playback_mute"]
                                ),
                            })}
                    >
                        {#if $device.mixer && $device.mixer["Speaker"] && $device.mixer["Speaker"]["playback_mute"]}
                            <VolumeX size={20} />
                        {:else}
                            <Volume2 size={20} />
                        {/if}
                    </button>
                    <div
                        class="flex-1 transition-opacity duration-200 {$device.mixer &&
                        $device.mixer['Speaker'] &&
                        $device.mixer['Speaker']['playback_mute']
                            ? 'opacity-40'
                            : 'opacity-100'}"
                    >
                        <div class="flex justify-between text-xs mb-1">
                            <span
                                class="text-gray-300 font-medium tracking-wide"
                            >
                                Speakers
                                {#if $device.mixer && $device.mixer["Speaker"] && $device.mixer["Speaker"]["playback_mute"]}
                                    <span class="text-red-500 font-bold ml-1"
                                        >[MUTED]</span
                                    >
                                {/if}
                            </span>
                            <span class="text-gray-500 font-mono">
                                {$device.mixer &&
                                $device.mixer["Speaker"] &&
                                $device.mixer["Speaker"]["playback_vol"] !==
                                    undefined
                                    ? (
                                          $device.mixer["Speaker"][
                                              "playback_vol"
                                          ] * 100
                                      ).toFixed(0)
                                    : 0}%
                            </span>
                        </div>
                        <input
                            type="range"
                            min="0"
                            max="1"
                            step="0.01"
                            value={$device.mixer && $device.mixer["Speaker"]
                                ? $device.mixer["Speaker"]["playback_vol"]
                                : 0}
                            on:input={(e) =>
                                device.updateMixer("Speaker", {
                                    playback_vol: parseFloat(
                                        e.currentTarget.value,
                                    ),
                                })}
                            class="w-full h-1.5 bg-gray-700 rounded-lg appearance-none cursor-pointer accent-blue-500 hover:accent-blue-400"
                        />
                    </div>
                </div>

                <!-- Global Mic Volume -->
                <div
                    class="flex-1 flex items-center space-x-4 w-full md:max-w-md"
                >
                    <button
                        class="p-2 rounded-lg transition-all duration-200 focus:outline-none {$device.mixer &&
                        $device.mixer['External Mic'] &&
                        $device.mixer['External Mic']['capture_mute']
                            ? 'bg-red-600/20 text-red-500 shadow-[0_0_10px_rgba(220,38,38,0.3)]'
                            : 'bg-red-600/10 text-red-500 hover:bg-red-600/20'}"
                        title={$device.mixer &&
                        $device.mixer["External Mic"] &&
                        $device.mixer["External Mic"]["capture_mute"]
                            ? "Unmute Microphone"
                            : "Mute Microphone"}
                        on:click={() =>
                            device.updateMixer("External Mic", {
                                capture_mute: !(
                                    $device.mixer &&
                                    $device.mixer["External Mic"] &&
                                    $device.mixer["External Mic"][
                                        "capture_mute"
                                    ]
                                ),
                            })}
                    >
                        {#if $device.mixer && $device.mixer["External Mic"] && $device.mixer["External Mic"]["capture_mute"]}
                            <MicOff size={20} />
                        {:else}
                            <Mic size={20} />
                        {/if}
                    </button>
                    <div
                        class="flex-1 transition-opacity duration-200 {$device.mixer &&
                        $device.mixer['External Mic'] &&
                        $device.mixer['External Mic']['capture_mute']
                            ? 'opacity-40'
                            : 'opacity-100'}"
                    >
                        <div class="flex justify-between text-xs mb-1">
                            <span
                                class="text-gray-300 font-medium tracking-wide"
                            >
                                Microphone
                                {#if $device.mixer && $device.mixer["External Mic"] && $device.mixer["External Mic"]["capture_mute"]}
                                    <span class="text-red-500 font-bold ml-1"
                                        >[MUTED]</span
                                    >
                                {/if}
                            </span>
                            <span class="text-gray-500 font-mono">
                                {$device.mixer &&
                                $device.mixer["External Mic"] &&
                                $device.mixer["External Mic"]["capture_vol"] !==
                                    undefined
                                    ? (
                                          $device.mixer["External Mic"][
                                              "capture_vol"
                                          ] * 100
                                      ).toFixed(0)
                                    : 0}%
                            </span>
                        </div>
                        <input
                            type="range"
                            min="0"
                            max="1"
                            step="0.01"
                            value={$device.mixer &&
                            $device.mixer["External Mic"]
                                ? $device.mixer["External Mic"]["capture_vol"]
                                : 0}
                            on:input={(e) =>
                                device.updateMixer("External Mic", {
                                    capture_vol: parseFloat(
                                        e.currentTarget.value,
                                    ),
                                })}
                            class="w-full h-1.5 bg-gray-700 rounded-lg appearance-none cursor-pointer accent-red-600 hover:accent-red-500"
                        />
                    </div>
                </div>
            </div>
        </footer>
    {/if}
</div>

<!-- Save Preset Dialog Overlay -->
{#if showSaveDialog}
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div
        class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
        on:click|self={() => (showSaveDialog = false)}
    >
        <div
            class="bg-[#1a1a1a] border border-[#333] rounded-2xl p-8 w-full max-w-md shadow-2xl shadow-black/50 relative"
        >
            <!-- Close Button -->
            <button
                class="absolute top-4 right-4 text-gray-500 hover:text-white transition-colors"
                on:click={() => (showSaveDialog = false)}
                aria-label="Close"
            >
                <X size={20} />
            </button>

            <!-- Header -->
            <div class="flex items-center space-x-3 mb-6">
                <div class="p-3 bg-green-600/10 rounded-xl text-green-500">
                    <Save size={24} />
                </div>
                <div>
                    <h3 class="font-bold text-white text-lg">Save EQ Preset</h3>
                    <p class="text-sm text-gray-500">
                        Save current equalizer settings as a custom preset
                    </p>
                </div>
            </div>

            <!-- Input -->
            <div class="mb-6">
                <label
                    for="preset_name"
                    class="block text-sm font-medium text-gray-400 mb-2"
                    >Preset Name</label
                >
                <input
                    id="preset_name"
                    type="text"
                    bind:value={newPresetName}
                    placeholder="e.g. My Custom Bass, Movie Night..."
                    class="w-full bg-[#111] border border-[#333] rounded-xl px-4 py-3 text-white placeholder-gray-600 focus:outline-none focus:border-green-500/50 focus:ring-1 focus:ring-green-500/30 transition-all text-sm"
                    on:keydown={(e) => {
                        if (e.key === "Enter") saveCustomPreset();
                    }}
                    autofocus
                />
            </div>

            <!-- Current EQ Preview -->
            <div class="mb-6 p-4 bg-[#111] rounded-xl border border-[#222]">
                <div
                    class="text-[10px] uppercase tracking-wider text-gray-600 mb-3 font-bold"
                >
                    Current EQ Values
                </div>
                <div class="flex items-end justify-between h-12 gap-0.5">
                    {#each eqBandsList as bandStr, i}
                        {@const val =
                            eqLocalValues[bandStr] !== undefined
                                ? eqLocalValues[bandStr]
                                : getFeature(bandStr)?.value?.Slider || 0}
                        {@const normalized = (val + 12) / 24}
                        <div class="flex-1 flex flex-col items-center">
                            <div
                                class="w-full rounded-sm transition-all"
                                style="height: {Math.max(
                                    2,
                                    normalized * 48,
                                )}px; background: linear-gradient(to top, #16a34a, #4ade80); opacity: {0.4 +
                                    normalized * 0.6};"
                            ></div>
                        </div>
                    {/each}
                </div>
                <div class="flex justify-between mt-1 text-[8px] text-gray-700">
                    <span>PRE</span>
                    <span>31</span>
                    <span>62</span>
                    <span>125</span>
                    <span>250</span>
                    <span>500</span>
                    <span>1k</span>
                    <span>2k</span>
                    <span>4k</span>
                    <span>8k</span>
                    <span>16k</span>
                </div>
            </div>

            <!-- Actions -->
            <div class="flex justify-end space-x-3">
                <button
                    class="px-5 py-2.5 bg-[#222] text-gray-400 rounded-xl text-sm font-medium hover:bg-[#2a2a2a] transition-colors"
                    on:click={() => (showSaveDialog = false)}
                >
                    Cancel
                </button>
                <button
                    class="px-5 py-2.5 bg-green-600 text-white rounded-xl text-sm font-bold hover:bg-green-500 transition-colors shadow-lg shadow-green-600/20 disabled:opacity-40 disabled:cursor-not-allowed flex items-center space-x-2"
                    on:click={saveCustomPreset}
                    disabled={!newPresetName.trim()}
                >
                    <Save size={16} />
                    <span>Save Preset</span>
                </button>
            </div>
        </div>
    </div>
{/if}

<style>
    /* Custom Scrollbar */
    ::-webkit-scrollbar {
        width: 8px;
    }
    ::-webkit-scrollbar-track {
        background: #111;
    }
    ::-webkit-scrollbar-thumb {
        background: #333;
        border-radius: 4px;
    }
    ::-webkit-scrollbar-thumb:hover {
        background: #444;
    }

    /* Dark theme for select dropdowns (WebView) */
    select,
    select option,
    select optgroup {
        background-color: #111 !important;
        color: #e5e5e5 !important;
    }
    select option:checked {
        background-color: #7f1d1d !important;
        color: #fff !important;
    }
    select option:hover {
        background-color: #222 !important;
    }
</style>
