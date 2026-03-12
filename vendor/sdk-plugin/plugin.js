export class AdiPlugin {
    dependencies = [];
    requires = [];
    #app;
    get app() {
        if (!this.#app) {
            throw new Error(`Plugin '${this.id}' accessed app before _init() was called`);
        }
        return this.#app;
    }
    /** Shorthand for this.app.bus. */
    get bus() {
        return this.app.bus;
    }
    /** @internal SDK use only. */
    async _init(app) {
        this.#app = app;
        const api = this['api'];
        if (api !== undefined)
            app._provide(this.id, api);
        await this.onRegister?.();
        app._registerPlugin(this.id);
        app.bus.emit('register-finished', { pluginId: this.id }, `plugin:${this.id}`);
    }
    /** @internal SDK use only. */
    async _destroy() {
        await this.onUnregister?.();
        this.#app?._unregisterPlugin(this.id);
    }
}
