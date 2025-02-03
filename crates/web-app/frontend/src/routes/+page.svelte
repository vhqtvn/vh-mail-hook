<script lang="ts">
import { auth } from '$lib/stores/auth';
</script>

<div class="min-h-[calc(100vh-4rem)]">
  <!-- Hero Section -->
  <div class="hero min-h-[70vh] bg-base-200">
    <div class="hero-content text-center">
      <div class="max-w-2xl">
        <h1 class="text-5xl font-bold mb-8">Disposable Email Made Simple</h1>
        <p class="text-xl mb-8">
          Create unlimited disposable email addresses to protect your privacy and keep your inbox clean.
          Perfect for signing up to services, testing, or avoiding spam.
        </p>
        {#if !$auth}
          <div class="flex flex-col sm:flex-row gap-4 justify-center">
            <a href="/auth/register" class="btn btn-primary">Get Started</a>
            <a href="/auth/login" class="btn btn-outline">Sign In</a>
          </div>
        {:else}
          <a href="/mailboxes" class="btn btn-primary">Go to Dashboard</a>
        {/if}
      </div>
    </div>
  </div>

  <!-- Features Section -->
  <div class="py-24 px-4">
    <div class="container mx-auto">
      <h2 class="text-3xl font-bold text-center mb-16">Why Choose Mail Hook?</h2>
      
      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-8">
        <!-- Privacy Feature -->
        <div class="card bg-base-200">
          <div class="card-body items-center text-center">
            <div class="w-16 h-16 mb-4 flex items-center justify-center rounded-full bg-primary/10">
              <svg xmlns="http://www.w3.org/2000/svg" class="h-8 w-8" viewBox="0 0 24 24" fill="none" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8V7a4 4 0 00-8 0v4h8z" />
              </svg>
            </div>
            <h3 class="text-xl font-bold mb-2">Privacy First</h3>
            <p>Keep your real email private and avoid spam by using disposable addresses for different services.</p>
          </div>
        </div>

        <!-- Unlimited Feature -->
        <div class="card bg-base-200">
          <div class="card-body items-center text-center">
            <div class="w-16 h-16 mb-4 flex items-center justify-center rounded-full bg-primary/10">
              <svg xmlns="http://www.w3.org/2000/svg" class="h-8 w-8" viewBox="0 0 24 24" fill="none" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
              </svg>
            </div>
            <h3 class="text-xl font-bold mb-2">Unlimited Mailboxes</h3>
            <p>Create as many disposable email addresses as you need, each with its own purpose and description.</p>
          </div>
        </div>

        <!-- Easy Management Feature -->
        <div class="card bg-base-200">
          <div class="card-body items-center text-center">
            <div class="w-16 h-16 mb-4 flex items-center justify-center rounded-full bg-primary/10">
              <svg xmlns="http://www.w3.org/2000/svg" class="h-8 w-8" viewBox="0 0 24 24" fill="none" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 10h16M4 14h16M4 18h16" />
              </svg>
            </div>
            <h3 class="text-xl font-bold mb-2">Easy Management</h3>
            <p>Simple interface to create, manage, and delete your disposable email addresses with just a few clicks.</p>
          </div>
        </div>
      </div>
    </div>
  </div>

  <!-- CTA Section -->
  <div class="bg-base-200 py-16">
    <div class="container mx-auto px-4 text-center">
      <h2 class="text-3xl font-bold mb-4">Ready to Get Started?</h2>
      <p class="text-xl mb-8">Join thousands of users who trust Mail Hook for their email privacy needs.</p>
      {#if !$auth}
        <div class="flex flex-col sm:flex-row gap-4 justify-center">
          <a href="/auth/register" class="btn btn-primary">Create Account</a>
          <a href="/auth/login" class="btn btn-outline">Sign In</a>
        </div>
      {:else}
        <a href="/mailboxes" class="btn btn-primary">Go to Dashboard</a>
      {/if}
    </div>
  </div>
</div>
