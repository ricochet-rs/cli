// Option to use ricochet-ui components directly via Leptos SSR
// This would require adding leptos as a dependency and serving actual components

use leptos::*;

// We could create Leptos components that match ricochet-ui's design
#[component]
pub fn AuthSuccessPage() -> impl IntoView {
    view! {
        <div class="min-h-screen bg-background flex items-center justify-center">
            <div class="bg-card border border-border p-12 max-w-md text-center">
                <div class="w-12 h-12 bg-success text-white rounded-full mx-auto mb-6 flex items-center justify-center">
                    "✓"
                </div>
                <h1 class="text-xl font-semibold mb-2">"Authentication Successful"</h1>
                <div class="inline-block px-4 py-1 bg-secondary text-secondary-foreground text-xs font-medium uppercase tracking-wider my-4">
                    "API Key Received"
                </div>
                <p class="text-muted-foreground text-sm my-4">
                    "Your CLI has been authenticated and is ready to use."
                </p>
                <div class="bg-primary text-primary-foreground px-4 py-2 font-mono text-xs my-6">
                    "$ ricochet --help"
                </div>
                <div class="mt-8 pt-6 border-t border-border text-muted-foreground text-xs">
                    "You can close this window and return to the CLI"
                </div>
            </div>
        </div>
    }
}

// To use this approach, we would need to:
// 1. Add leptos dependencies to Cargo.toml
// 2. Set up Leptos SSR in our axum callback server
// 3. Render the component to HTML string
// 4. Include the ricochet-ui CSS files