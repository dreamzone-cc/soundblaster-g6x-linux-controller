// @ts-nocheck
import { writable } from 'svelte/store';

function createDeviceStore() {
    const { subscribe, set, update } = writable({
        features: [],
        eq_bands: Array(11).fill(0),
        mixer: {},
        loading: true,
        error: null,
    });

    let pollInterval;
    let pendingUpdate = false; // Block polling during updates
    let debounceTimer; // Timer for debouncing forceLoad

    const load = async () => {
        // Skip poll if we have a pending update (prevents stale data overwriting optimistic UI)
        if (pendingUpdate) return;

        try {
            const url = import.meta.env.DEV ? 'http://localhost:3311/api/status' : '/api/status';
            const mixerUrl = import.meta.env.DEV ? 'http://localhost:3311/api/mixer/status' : '/api/mixer/status';

            const [res, mixerRes] = await Promise.all([
                fetch(url).catch(() => null),
                fetch(mixerUrl).catch(() => null)
            ]);

            if (!res || !res.ok) throw new Error('Failed to fetch status');

            const data = await res.json();
            const mixerData = mixerRes && mixerRes.ok ? await mixerRes.json() : {};

            // Double-check we're still not in a pending state
            if (!pendingUpdate) {
                update(s => ({ ...s, ...data, mixer: mixerData, loading: false, error: null }));
            }
        } catch (err) {
            console.error(err);
            update(s => ({
                ...s,
                error: err.message || "Connection Error",
                loading: false,
                features: s.features && s.features.length > 0 ? s.features : [
                    { name: "SBX", value: { Toggle: true } },
                    { name: "Surround", value: { Toggle: true }, dependencies: ["SBX"] },
                    { name: "Bass", value: { Toggle: true }, dependencies: ["SBX"] }
                ]
            }));
        }
    };

    // Force reload that bypasses the pendingUpdate guard
    const forceLoad = async () => {
        pendingUpdate = false;
        await load();
    };

    // Debounced forceLoad to avoid interrupting fast slider dragging
    const debouncedForceLoad = () => {
        if (debounceTimer) clearTimeout(debounceTimer);
        debounceTimer = setTimeout(forceLoad, 500); // Wait 500ms after last interaction
    };

    return {
        subscribe,
        load,
        startPolling: (interval = 2000) => {
            if (pollInterval) clearInterval(pollInterval);
            load(); // Initial load
            pollInterval = setInterval(load, interval);
        },
        stopPolling: () => {
            if (pollInterval) clearInterval(pollInterval);
        },
        updateFeature: async (name, value) => {
            // Block polling from overwriting our optimistic update
            pendingUpdate = true;

            // Optimistic UI update
            update(state => {
                const newFeatures = state.features.map(f => {
                    let newValue = f.value;

                    if (f.name === name) {
                        if (typeof value === 'boolean') newValue = { Toggle: value };
                        if (typeof value === 'number') newValue = { Slider: value };
                    }

                    // SBX and Scout Mode are mutually exclusive
                    if (typeof value === 'boolean' && value === true) {
                        if (name === "SBX" && f.name === "Scout Mode") {
                            newValue = { Toggle: false };
                        }
                        if (name === "Scout Mode" && f.name === "SBX") {
                            newValue = { Toggle: false };
                        }
                    }

                    return { ...f, value: newValue };
                });
                return { ...state, features: newFeatures };
            });

            try {
                const payload = {};
                payload.name = name;
                if (typeof value === 'boolean') payload.toggle = value;
                if (typeof value === 'number') payload.slider = value;

                const url = import.meta.env.DEV ? 'http://localhost:3311/api/feature' : '/api/feature';
                await fetch(url, {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify(payload)
                });

                // Re-fetch authoritative state after server has processed (debounced)
                debouncedForceLoad();
            } catch (err) {
                console.error('Failed to update feature', err);
                pendingUpdate = false;
            }
        },
        updateMixer: async (name, payload) => {
            pendingUpdate = true;

            update(state => {
                const newMixer = { ...state.mixer };
                if (!newMixer[name]) newMixer[name] = {};
                newMixer[name] = { ...newMixer[name], ...payload };
                return { ...state, mixer: newMixer };
            });

            try {
                const url = import.meta.env.DEV ? 'http://localhost:3311/api/mixer/feature' : '/api/mixer/feature';
                await fetch(url, {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ name, ...payload })
                });

                // Re-fetch after server processed (debounced)
                debouncedForceLoad();
            } catch (err) {
                console.error('Failed to update mixer', err);
                pendingUpdate = false;
            }
        }
    };
}

export const device = createDeviceStore();

