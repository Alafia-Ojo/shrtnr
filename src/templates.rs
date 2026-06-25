pub const INDEX_HTML: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>URL Shortener</title>
    <script src="https://unpkg.com/htmx.org@2"></script>
    <script>
      function copy(url, btn) {
        navigator.clipboard.writeText(url);
        btn.textContent = 'copied';
        setTimeout(() => btn.textContent = 'copy', 1500);
      }
    </script>
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link href="https://fonts.googleapis.com/css2?family=Inter:opsz@14..32&display=swap" rel="stylesheet">
  <style>
    * { box-sizing: border-box; margin: 0; padding: 0; }
    body {
      font-family: 'Inter', system-ui, sans-serif;
      background: linear-gradient(135deg, #0f172a 0%, #1e293b 100%);
      min-height: 100vh;
      display: flex;
      align-items: center;
      justify-content: center;
      padding: 1rem;
      color: #e2e8f0;
    }
    .container {
      background: #1e293b;
      border: 1px solid #334155;
      border-radius: 16px;
      padding: 2.5rem;
      width: 100%;
      max-width: 480px;
      box-shadow: 0 25px 50px -12px rgba(0,0,0,0.5);
    }
    @media (max-width: 480px) {
      .container { padding: 1.5rem; }
    }
    .logo {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      margin-bottom: 0.5rem;
    }
    .logo svg { flex-shrink: 0; }
    h1 {
      font-size: 1.5rem;
      font-weight: 600;
      letter-spacing: -0.02em;
    }
    .subtitle {
      color: #94a3b8;
      font-size: 0.875rem;
      margin-bottom: 1.5rem;
    }
    .input-group {
      display: flex;
      flex-direction: column;
      gap: 0.5rem;
    }
    .input-group + .input-group { margin-top: 1rem; }
    label {
      font-size: 0.875rem;
      font-weight: 500;
      color: #cbd5e1;
    }
    .input-wrap {
      display: flex;
      gap: 0.5rem;
    }
    @media (max-width: 400px) {
      .input-wrap { flex-direction: column; }
    }
    .input-wrap input {
      flex: 1;
      width: 100%;
      padding: 0.75rem 1rem;
      font-size: 0.9375rem;
      font-family: inherit;
      background: #0f172a;
      border: 1px solid #334155;
      border-radius: 10px;
      color: #e2e8f0;
      outline: none;
      transition: border-color 0.2s;
    }
    .input-wrap input:focus {
      border-color: #6366f1;
    }
    .input-wrap input::placeholder {
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
      transition: opacity 0.2s, transform 0.1s;
      white-space: nowrap;
    }
    .input-wrap button:hover { opacity: 0.9; }
    .input-wrap button:active { transform: scale(0.97); }
    .card {
      margin-top: 1.5rem;
      background: #0f172a;
      border: 1px solid #334155;
      border-radius: 12px;
      padding: 1.25rem;
    }
    .card .short-url {
      font-size: 1.125rem;
      font-weight: 600;
      word-break: break-all;
    }
    .card .short-url a {
      color: #a5b4fc;
      text-decoration: none;
    }
    .card .short-url a:hover { text-decoration: underline; }
    .card .short-url { display: flex; align-items: center; gap: 0.5rem; }
    .copy-btn {
      background: #334155;
      border: none;
      color: #94a3b8;
      font-size: 0.6875rem;
      font-family: inherit;
      font-weight: 500;
      padding: 0.25rem 0.5rem;
      border-radius: 6px;
      cursor: pointer;
      transition: background 0.15s, color 0.15s;
      flex-shrink: 0;
    }
    .copy-btn:hover { background: #475569; color: #e2e8f0; }
    .card .meta {
      margin-top: 0.75rem;
      font-size: 0.8125rem;
      color: #64748b;
      display: flex;
      align-items: center;
      gap: 0.75rem;
    }
    .card .meta a {
      color: #818cf8;
      text-decoration: none;
    }
    .card .meta a:hover { text-decoration: underline; }
    .card .meta .sep { color: #334155; }
    .error {
      color: #fca5a5;
    }
    .original-link {
      font-size: 0.8125rem;
      color: #64748b;
      margin-top: 0.5rem;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }
    .original-link a {
      color: #64748b;
    }
    .footer {
      margin-top: 2rem;
      text-align: center;
      font-size: 0.75rem;
      color: #475569;
    }
    .footer a { color: #6366f1; text-decoration: none; }
    .optional { color: #64748b; font-weight: 400; }
    .code-input {
      width: 100%;
      padding: 0.75rem 1rem;
      font-size: 0.9375rem;
      font-family: inherit;
      background: #0f172a;
      border: 1px solid #334155;
      border-radius: 10px;
      color: #e2e8f0;
      outline: none;
      transition: border-color 0.2s;
    }
    .code-input:focus { border-color: #6366f1; }
    .code-input::placeholder { color: #475569; }
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
      border-radius: 8px;
      background: #e2e8f0;
      padding: 8px;
    }
  </style>
</head>
<body>
  <div class='container'>
    <div class='logo'>
      <svg width='28' height='28' viewBox='0 0 24 24' fill='none' stroke='#818cf8' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'>
        <path d='M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71'/>
        <path d='M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71'/>
      </svg>
      <h1>Shrtnr</h1>
    </div>
    <p class='subtitle'>Paste a long URL and get a short link</p>
    <form hx-post='/shorten' hx-target='#result' hx-swap='innerHTML' hx-on::after-request="this.reset()">
      <div class='input-group'>
        <label for='url'>URL to shorten</label>
        <div class='input-wrap'>
          <input type='text' id='url' name='url' placeholder='https://example.com/very/long/url' required autofocus>
          <button type='submit'>Shorten</button>
        </div>
      </div>
      <input type='hidden' name='creator_id' value='__CREATOR_ID__'>
      <div class='input-group'>
        <label for='code'>Custom code <span class='optional'>(optional)</span></label>
        <input class='code-input' type='text' id='code' name='code' placeholder='my-custom-link' maxlength='20'>
      </div>
    </form>
    <div id='result'></div>
    <div class='footer'><a href='/dashboard'>Dashboard</a> · Powered by <a href='https://www.rust-lang.org/'>Rust</a> + <a href='https://htmx.org/'>HTMX</a></div>
  </div>
</body>
</html>"##;
