// Browser facilities missing from jsdom, used by Glisse and desktop surfaces.
export function installBrowserMocks(window) {
  window.matchMedia = () => ({ matches: false, addListener() {}, removeListener() {}, addEventListener() {}, removeEventListener() {} });
  window.ResizeObserver = class { observe() {} unobserve() {} disconnect() {} };
  // Glisse attaches switch handlers in an effect. Expose actual readiness so
  // interaction tests do not race mounting or rely on arbitrary timeouts.
  const readySwitches = new WeakSet();
  const addListener = window.EventTarget.prototype.addEventListener;
  window.EventTarget.prototype.addEventListener = function (type, listener, options) {
    if (type === "click" && this instanceof window.Element && this.classList.contains("lg-switch")) readySwitches.add(this);
    return addListener.call(this, type, listener, options);
  };
  window.glisseSwitchReady = element => !!element && readySwitches.has(element);
}
