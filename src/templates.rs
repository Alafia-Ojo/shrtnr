pub const INDEX_HTML: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Shrtnr</title>
  <link rel="icon" type="image/x-icon" href="/favicon.ico">
  <script src="https://unpkg.com/htmx.org@2"></script>
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link href="https://fonts.googleapis.com/css2?family=Inter:opsz@14..32&display=swap" rel="stylesheet">
  <style>
    * { box-sizing: border-box; margin: 0; padding: 0; }
    body {
      font-family: 'Inter', system-ui, sans-serif;
      background: #0b1120;
      min-height: 100vh;
      display: flex;
      align-items: center;
      justify-content: center;
      padding: 1rem;
      color: #e2e8f0;
    }
    body::before {
      content: '';
      position: fixed; inset: 0; z-index: -1;
      background:
        radial-gradient(600px circle at 50% 0%, rgba(99,102,241,0.08) 0%, transparent 70%),
        radial-gradient(400px circle at 80% 80%, rgba(139,92,246,0.06) 0%, transparent 60%);
    }
    .container {
      background: rgba(30,41,59,0.7);
      backdrop-filter: blur(16px);
      -webkit-backdrop-filter: blur(16px);
      border: 1px solid rgba(51,65,85,0.5);
      border-radius: 20px;
      padding: 2.5rem;
      width: 100%;
      max-width: 480px;
      box-shadow: 0 25px 50px -12px rgba(0,0,0,0.5), inset 0 1px 0 rgba(255,255,255,0.03);
      transition: box-shadow 0.3s;
    }
    .container:focus-within {
      box-shadow: 0 25px 50px -12px rgba(99,102,241,0.15), inset 0 1px 0 rgba(255,255,255,0.03);
    }
    @media (max-width: 480px) {
      .container { padding: 1.5rem; border-radius: 16px; }
    }
    .logo {
      display: flex;
      align-items: center;
      gap: 0.625rem;
      margin-bottom: 0.375rem;
    }
    .logo svg { flex-shrink: 0; filter: drop-shadow(0 0 8px rgba(129,140,248,0.3)); }
    h1 {
      font-size: 1.5rem;
      font-weight: 700;
      letter-spacing: -0.03em;
      background: linear-gradient(135deg, #e2e8f0 0%, #94a3b8 100%);
      -webkit-background-clip: text;
      -webkit-text-fill-color: transparent;
      background-clip: text;
    }
    .subtitle {
      color: #64748b;
      font-size: 0.875rem;
      margin-bottom: 1.5rem;
    }
    .input-group {
      display: flex;
      flex-direction: column;
      gap: 0.5rem;
    }
    .input-group + .input-group { margin-top: 1.5rem; }
    label {
      font-size: 0.9rem;
      font-weight: 500;
      color: #94a3b8;
      letter-spacing: 0.01em;
      padding-top: 2rem;
    }
    .input-wrap {
      display: flex;
      gap: 0.5rem;
    }
    @media (max-width: 400px) {
      .input-wrap { flex-direction: column; }
    }
    .input-wrap input, .code-input {
      flex: 1;
      width: 100%;
      padding: 0.75rem 1rem;
      font-size: 0.9375rem;
      font-family: inherit;
      background: rgba(15,23,42,0.6);
      border: 1px solid rgba(51,65,85,0.6);
      border-radius: 10px;
      color: #e2e8f0;
      outline: none;
      transition: border-color 0.25s, box-shadow 0.25s;
    }
    .input-wrap input:focus, .code-input:focus {
      border-color: rgba(99,102,241,0.5);
      box-shadow: 0 0 0 3px rgba(99,102,241,0.1);
    }
    .input-wrap input::placeholder, .code-input::placeholder {
      color: #475569;
    }
    .input-wrap button {
      padding: 0.75rem 1.25rem;
      font-size: 0.9375rem;
      font-family: inherit;
      font-weight: 500;
      background: linear-gradient(135deg, #6366f1, #8b5cf6);
      color: #fff;
      border: none;
      border-radius: 10px;
      cursor: pointer;
      transition: opacity 0.2s, transform 0.15s, box-shadow 0.2s;
      white-space: nowrap;
    }
    .input-wrap button:hover {
      opacity: 0.92;
      box-shadow: 0 4px 20px rgba(99,102,241,0.3);
    }
    .input-wrap button:active { transform: scale(0.97); }
    .card {
      margin-top: 1.5rem;
      background: rgba(15,23,42,0.8);
      backdrop-filter: blur(8px);
      -webkit-backdrop-filter: blur(8px);
      border: 1px solid rgba(51,65,85,0.4);
      border-radius: 14px;
      padding: 1.25rem;
      animation: card-in 0.35s ease;
    }
    @keyframes card-in {
      from { opacity: 0; transform: translateY(8px); }
      to { opacity: 1; transform: translateY(0); }
    }
    .card .short-url {
      font-size: 1.125rem;
      font-weight: 600;
      word-break: break-all;
      display: flex;
      align-items: center;
      gap: 0.5rem;
    }
    .card .short-url a {
      color: #a5b4fc;
      text-decoration: none;
    }
    .card .short-url a:hover { text-decoration: underline; }
    .copy-btn {
      background: rgba(51,65,85,0.6);
      border: 1px solid rgba(71,85,105,0.3);
      color: #94a3b8;
      font-size: 0.6875rem;
      font-family: inherit;
      font-weight: 500;
      padding: 0.25rem 0.625rem;
      border-radius: 6px;
      cursor: pointer;
      transition: background 0.15s, color 0.15s, border-color 0.15s;
      flex-shrink: 0;
    }
    .copy-btn:hover { background: rgba(71,85,105,0.6); color: #e2e8f0; border-color: rgba(99,102,241,0.3); }
    .card .meta {
      margin-top: 0.75rem;
      font-size: 0.8125rem;
      color: #64748b;
      display: flex;
      align-items: center;
      gap: 0.75rem;
      flex-wrap: wrap;
    }
    .card .meta a {
      color: #818cf8;
      text-decoration: none;
    }
    .card .meta a:hover { text-decoration: underline; }
    .card .meta .sep { color: #334155; }
    .error {
      color: #fca5a5;
      font-size: 0.875rem;
    }
    .original-link {
      font-size: 0.8125rem;
      color: #64748b;
      margin-top: 0.5rem;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }
    .original-link a { color: #64748b; }
    .footer {
      margin-top: 2rem;
      text-align: center;
      font-size: 0.75rem;
      color: #475569;
    }
    .footer a { color: #6366f1; text-decoration: none; }
    .footer a:hover { text-decoration: underline; }
    .optional { color: #64748b; font-weight: 400; }
    .code-input { margin-top: 0; }
    select.code-input { cursor: pointer; appearance: none; background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' fill='%2364748b' viewBox='0 0 16 16'%3E%3Cpath d='M8 11L3 6h10z'/%3E%3C/svg%3E"); background-repeat: no-repeat; background-position: right 0.75rem center; padding-right: 2rem; }
    .qr-download { text-align: center; margin: 0.25rem 0 0.5rem; }
    .qr-download a { color: #64748b; font-size: 0.75rem; text-decoration: none; }
    .qr-download a:hover { color: #818cf8; text-decoration: underline; }
    .qr-wrap {
      display: flex;
      justify-content: center;
      margin: 1rem 0 0.5rem;
    }
    .qr-wrap svg {
      width: 140px;
      height: 140px;
      border-radius: 10px;
      background: #e2e8f0;
      padding: 8px;
    }
    .loading-bar {
      position: fixed; top: 0; left: 0; z-index: 9999;
      width: 0; height: 3px;
      background: linear-gradient(90deg, #6366f1, #8b5cf6, #a78bfa);
      border-radius: 0 2px 2px 0;
      transition: width 0.3s ease, opacity 0.3s;
      opacity: 0;
      box-shadow: 0 0 12px rgba(99,102,241,0.5);
    }
    .loading-bar.active { width: 60%; opacity: 1; }
    .loading-bar.done { width: 100%; opacity: 0; transition: width 0.15s, opacity 0.4s 0.15s; }
    @keyframes fade-in {
      from { opacity: 0; }
      to { opacity: 1; }
    }
    #result.htmx-added { animation: fade-in 0.3s ease; }
    .toast-container {
      position: fixed; top: 1rem; right: 1rem; z-index: 999;
      display: flex; flex-direction: column; gap: 0.5rem;
      pointer-events: none;
    }
    .toast {
      pointer-events: auto;
      background: rgba(30,41,59,0.95);
      backdrop-filter: blur(12px);
      -webkit-backdrop-filter: blur(12px);
      border: 1px solid rgba(51,65,85,0.5);
      border-radius: 10px;
      padding: 0.75rem 1rem;
      font-size: 0.875rem;
      color: #e2e8f0;
      box-shadow: 0 10px 30px -10px rgba(0,0,0,0.5);
      animation: toast-in 0.3s ease, toast-out 0.3s ease 3.7s forwards;
      max-width: 360px;
    }
    .toast.error { border-color: rgba(239,68,68,0.5); }
    @keyframes toast-in {
      from { opacity: 0; transform: translateX(100%); }
      to { opacity: 1; transform: translateX(0); }
    }
    @keyframes toast-out {
      from { opacity: 1; }
      to { opacity: 0; transform: translateX(100%); }
    }
  </style>
</head>
<body>
  <div class="loading-bar" id="loading-bar"></div>
  <div class="toast-container" id="toast-container"></div>
  <div class="container">
    <div class="logo">
      <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="#818cf8" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"/>
        <path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"/>
      </svg>
      <h1>Shrtnr</h1>
    </div>
    <p class="subtitle">Paste a long URL and get a short link</p>
    <form hx-post="/shorten" hx-target="#result" hx-swap="innerHTML" hx-on::after-request="this.reset()">
      <div class="input-group">
        <label for="url">URL to shorten</label>
        <div class="input-wrap">
          <input type="text" id="url" name="url" placeholder="https://example.com/very/long/url" required autofocus>
          <button type="submit">Shorten</button>
        </div>
      </div>
      <input type="hidden" name="creator_id" value="__CREATOR_ID__">
      <div class="input-group">
        <label for="code">Custom code <span class="optional">(optional)</span></label>
        <input class="code-input" type="text" id="code" name="code" placeholder="my-custom-link" maxlength="20">
      </div>
      <div class="input-group">
        <label for="expiry">Link expires</label>
        <select class="code-input" id="expiry" name="expiry">
          <option value="">Never</option>
          <option value="30">30 Minutes</option>
          <option value="60">1 Hour</option>
          <option value="120">2 Hours</option>
          <option value="360">6 Hours</option>
          <option value="720">12 Hours</option>
          <option value="1440">1 Day</option>
          <option value="4320">3 Days</option>
          <option value="10080">1 Week</option>
          <option value="43200">30 Days</option>
        </select>
      </div>
    </form>
    <div id="result"></div>
    <div class="footer"><a href="/dashboard">Dashboard</a> &middot; Powered by <a href="https://www.rust-lang.org/">Rust</a> + <a href="https://htmx.org/">HTMX</a></div>
  </div>
  <script>
    function copy(url, btn) {
      navigator.clipboard.writeText(url);
      btn.textContent = 'copied';
      btn.style.borderColor = 'rgba(52,211,153,0.4)';
      btn.style.color = '#34d399';
      setTimeout(() => {
        btn.textContent = 'copy';
        btn.style.borderColor = '';
        btn.style.color = '';
      }, 1500);
    }
    function showToast(msg, type) {
      var c = document.getElementById('toast-container');
      if (!c) return;
      var t = document.createElement('div');
      t.className = 'toast' + (type ? ' ' + type : '');
      t.textContent = msg;
      c.appendChild(t);
      setTimeout(function() { if (t.parentNode) t.parentNode.removeChild(t); }, 4200);
    }
    var loadBar = document.getElementById('loading-bar');
    document.body.addEventListener('htmx:beforeRequest', function() {
      loadBar.className = 'loading-bar active';
    });
    document.body.addEventListener('htmx:afterRequest', function() {
      loadBar.className = 'loading-bar done';
      setTimeout(function() { loadBar.className = 'loading-bar'; }, 600);
    });
    document.body.addEventListener('htmx:beforeSwap', function(evt) {
      if (evt.detail.xhr && evt.detail.xhr.status >= 400) {
        showToast(evt.detail.serverResponse || 'Request failed', 'error');
        evt.detail.shouldSwap = false;
      }
    });
  </script>
</body>
</html>"##;
