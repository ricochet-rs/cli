// HTML pages for OAuth authentication callbacks

// Minimal embedded CSS for auth pages
const RICOCHET_UI_CSS: &str = r#"
:root {
    --background: #ffffff;
    --foreground: #09090b;
    --card: #ffffff;
    --card-foreground: #09090b;
    --border: #e4e4e7;
    --primary: #18181b;
    --primary-foreground: #fafafa;
    --secondary: #f4f4f5;
    --secondary-foreground: #18181b;
    --muted: #f4f4f5;
    --muted-foreground: #71717a;
    --destructive: #ef4444;
    --destructive-foreground: #fafafa;
    --font-sans: system-ui, -apple-system, sans-serif;
    --font-mono: ui-monospace, monospace;
}

@media (prefers-color-scheme: dark) {
    :root {
        --background: #09090b;
        --foreground: #fafafa;
        --card: #18181b;
        --card-foreground: #fafafa;
        --border: #27272a;
        --primary: #fafafa;
        --primary-foreground: #18181b;
        --secondary: #27272a;
        --secondary-foreground: #fafafa;
        --muted: #27272a;
        --muted-foreground: #a1a1aa;
    }
}

* {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
}

body {
    margin: 0;
    font-family: var(--font-sans);
}
"#;

pub fn create_success_page() -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Success - Ricochet CLI</title>
    <style>
        /* Include ricochet-ui styles */
        {}

        /* Page-specific styles */
        .auth-container {{
            min-height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
            background: var(--background);
            color: var(--foreground);
            font-family: var(--font-sans);
        }}

        .auth-card {{
            background: var(--card);
            border: 1px solid var(--border);
            padding: 3rem;
            max-width: 24rem;
            text-align: center;
            position: relative;
        }}

        .success-icon {{
            width: 3rem;
            height: 3rem;
            line-height: 3rem;
            margin: 0 auto 1.5rem;
            background: oklch(0.62 0.19 142);
            color: white;
            display: flex;
            align-items: center;
            justify-content: center;
            font-size: 1.5rem;
        }}

        .auth-title {{
            font-size: 1.25rem;
            font-weight: 600;
            margin-bottom: 0.5rem;
            color: var(--card-foreground);
        }}

        .auth-badge {{
            display: inline-block;
            padding: 0.25rem 1rem;
            background: var(--secondary);
            color: var(--secondary-foreground);
            font-size: 0.75rem;
            font-weight: 500;
            margin: 1rem 0;
            letter-spacing: 0.05em;
            text-transform: uppercase;
        }}

        .auth-message {{
            color: var(--muted-foreground);
            margin: 1rem 0;
            font-size: 0.875rem;
            line-height: 1.5;
        }}

        .terminal-hint {{
            font-family: var(--font-mono);
            background: var(--primary);
            color: var(--primary-foreground);
            padding: 0.5rem 1rem;
            margin: 1.5rem 0;
            font-size: 0.75rem;
        }}

        .close-hint {{
            margin-top: 2rem;
            padding-top: 1.5rem;
            border-top: 1px solid var(--border);
            color: var(--muted-foreground);
            font-size: 0.75rem;
        }}

        /* Dark mode detection */
        @media (prefers-color-scheme: dark) {{
            html {{
                color-scheme: dark;
            }}
        }}
    </style>
</head>
<body class="auth-container">
    <div class="auth-card">
        <div class="success-icon">✓</div>
        <h1 class="auth-title">Authentication Successful</h1>
        <div class="auth-badge">API Key Received</div>
        <p class="auth-message">Your CLI has been authenticated and is ready to use.</p>
        <div class="close-hint">You can close this window and return to the CLI</div>
    </div>
</body>
</html>"#,
        RICOCHET_UI_CSS
    )
}

pub fn create_error_page(error: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Error - Ricochet CLI</title>
    <style>
        {}

        .auth-container {{
            min-height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
            background: var(--background);
            color: var(--foreground);
            font-family: var(--font-sans);
        }}

        .auth-card {{
            background: var(--card);
            border: 1px solid var(--border);
            padding: 3rem;
            max-width: 24rem;
            text-align: center;
        }}

        .error-icon {{
            width: 3rem;
            height: 3rem;
            line-height: 3rem;
            margin: 0 auto 1.5rem;
            background: var(--destructive);
            color: var(--destructive-foreground);
            display: flex;
            align-items: center;
            justify-content: center;
            font-size: 1.5rem;
        }}

        .auth-title {{
            font-size: 1.25rem;
            font-weight: 600;
            margin-bottom: 0.5rem;
            color: var(--card-foreground);
        }}

        .error-message {{
            background: var(--secondary);
            color: var(--secondary-foreground);
            padding: 0.75rem;
            margin: 1rem 0;
            font-family: var(--font-mono);
            font-size: 0.75rem;
            word-break: break-all;
        }}

        .auth-message {{
            color: var(--muted-foreground);
            margin: 1rem 0;
            font-size: 0.875rem;
        }}

        .close-hint {{
            margin-top: 2rem;
            padding-top: 1.5rem;
            border-top: 1px solid var(--border);
            color: var(--muted-foreground);
            font-size: 0.75rem;
        }}

        @media (prefers-color-scheme: dark) {{
            html {{
                color-scheme: dark;
            }}
        }}
    </style>
</head>
<body class="auth-container">
    <div class="auth-card">
        <div class="error-icon">×</div>
        <h1 class="auth-title">Authentication Failed</h1>
        <div class="error-message">{}</div>
        <p class="auth-message">Please return to the CLI and try again.</p>
        <div class="close-hint">You can close this window</div>
    </div>
</body>
</html>"#,
        RICOCHET_UI_CSS,
        html_escape::encode_text(error)
    )
}

pub fn create_session_page() -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Success - Ricochet CLI</title>
    <style>
        {}

        .auth-container {{
            min-height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
            background: var(--background);
            color: var(--foreground);
            font-family: var(--font-sans);
        }}

        .auth-card {{
            background: var(--card);
            border: 1px solid var(--border);
            padding: 3rem;
            max-width: 24rem;
            text-align: center;
        }}

        .success-icon {{
            width: 3rem;
            height: 3rem;
            line-height: 3rem;
            margin: 0 auto 1.5rem;
            background: oklch(0.62 0.19 142);
            color: white;
            display: flex;
            align-items: center;
            justify-content: center;
            font-size: 1.5rem;
        }}

        .auth-title {{
            font-size: 1.25rem;
            font-weight: 600;
            margin-bottom: 0.5rem;
            color: var(--card-foreground);
        }}

        .auth-badge {{
            display: inline-block;
            padding: 0.25rem 1rem;
            background: var(--secondary);
            color: var(--secondary-foreground);
            font-size: 0.75rem;
            font-weight: 500;
            margin: 1rem 0;
            letter-spacing: 0.05em;
            text-transform: uppercase;
        }}

        .auth-message {{
            color: var(--muted-foreground);
            margin: 1rem 0;
            font-size: 0.875rem;
        }}

        .close-hint {{
            margin-top: 2rem;
            padding-top: 1.5rem;
            border-top: 1px solid var(--border);
            color: var(--muted-foreground);
            font-size: 0.75rem;
        }}

        @media (prefers-color-scheme: dark) {{
            html {{
                color-scheme: dark;
            }}
        }}
    </style>
</head>
<body class="auth-container">
    <div class="auth-card">
        <div class="success-icon">✓</div>
        <h1 class="auth-title">Authentication Successful</h1>
        <div class="auth-badge">Session Established</div>
        <p class="auth-message">Your CLI session has been authenticated.</p>
        <div class="close-hint">You can close this window and return to the CLI</div>
    </div>
</body>
</html>"#,
        RICOCHET_UI_CSS
    )
}
