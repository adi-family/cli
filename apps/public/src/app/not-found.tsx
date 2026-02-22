"use client";

import Link from "next/link";

export default function RootNotFound() {
  return (
    <html lang="en" className="dark">
      <body className="min-h-screen bg-[#0a0a0a] text-[#a0a0a0] antialiased">
        <div className="flex min-h-screen flex-col items-center justify-center px-6 text-center">
          <span className="font-mono text-6xl font-bold" style={{color: "rgba(135,95,215,0.2)"}}>404</span>
          <h1 className="mt-4 text-2xl font-semibold text-[#e0e0e0]">
            Page not found
          </h1>
          <p className="mt-2">
            The page you&apos;re looking for doesn&apos;t exist or has been moved.
          </p>
          <Link
            href="/"
            className="mt-8 inline-flex items-center rounded-full border border-[rgba(255,255,255,0.07)] px-6 py-2.5 text-sm hover:text-[#e0e0e0] transition-colors"
          >
            &larr; Back to home
          </Link>
        </div>
      </body>
    </html>
  );
}
