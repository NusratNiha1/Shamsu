Create a **stunning, premium, glassmorphic To-Do application** using **React (Vite)**, **TypeScript**, **Tailwind CSS**, **Framer Motion**, **React Icons**, and **React Hot Toast**. The application should feel like a luxury macOS/iOS productivity app with an emphasis on **glassmorphism, grainy textures, layered translucent panels, vibrant gradients, soft lighting, and buttery-smooth animations**.

## Overall Design Style

Design language should combine:

* Apple macOS Sonoma aesthetics
* Glassmorphism
* Frosted glass
* Layered translucent cards
* Grain/noise overlay
* Soft ambient lighting
* Large rounded corners
* Floating panels
* Beautiful depth
* Minimal but luxurious interface

The UI should look like something from **Linear, Arc Browser, Raycast, Notion Calendar, and Apple's Human Interface Guidelines**, while maintaining its own identity.

---

# Visual Style

Background:

* Large blurred gradient blobs
* Aurora gradients
* Mesh gradients
* Floating radial lights
* Noise/grain overlay
* Soft vignette
* Animated gradient movement (very slow)

Example colors:

* Indigo
* Violet
* Blue
* Cyan
* Emerald
* Pink

Background should never be plain.

---

# Glassmorphism

Every major component should use glass effects.

Example styling:

* backdrop-filter: blur(24px)
* backdrop-blur-2xl
* bg-white/10
* dark:bg-white/5
* border-white/20
* shadow-2xl
* shadow-black/10
* saturate(180%)

Panels should appear layered with depth.

---

# Grain Texture

Add subtle grain/noise over the entire application.

Requirements:

* CSS noise overlay
* Very low opacity (3–6%)
* Blend naturally
* Should resemble premium UI found in macOS and Figma

Implement using CSS pseudo-elements instead of image assets whenever possible.

Example:

* SVG turbulence filter
* CSS generated noise
* Tiny repeating SVG texture
* Mix-blend-mode overlay

The grain should be almost invisible but noticeable.

---

# Lighting

Create realistic lighting.

Include:

* Soft inner highlights
* Ambient glow
* Gradient borders
* Glass reflections
* Floating shadows
* Radial light sources

Cards should appear illuminated.

---

# Borders

Use subtle gradient borders.

Examples:

* White 20% opacity
* Purple glow
* Blue glow
* Frosted edge

Never use harsh borders.

---

# Shadows

Use layered shadows.

Example:

* Shadow under glass
* Inner shadow
* Ambient shadow
* Colored glow shadow

Cards should appear floating.

---

# Animations

Use Framer Motion extensively.

Animations:

* Glass cards fade in
* Floating hover effect
* Magnetic buttons
* Smooth scaling
* Glow animation
* Gradient movement
* Page transitions
* Blur transitions
* Staggered task appearance
* Button ripple
* Hover tilt
* Spring animations

All animations should run at 60 FPS.

---

# Typography

Use modern fonts.

Examples:

* Inter
* Plus Jakarta Sans
* Geist

Typography should be clean, spacious, and premium.

---

# Components

Each component should look premium.

Examples:

Glass Card

* Frosted
* Layered
* Rounded (24px)
* Floating

Buttons

* Gradient
* Glow on hover
* Glass surface
* Soft shadow

Inputs

* Frosted glass
* Blur
* Transparent
* Animated focus ring

Sidebar

* Floating
* Glass
* Collapsible
* Blur background

Navbar

* Sticky
* Transparent
* Glass effect

Task Card

* Floating
* Smooth hover lift
* Glass background
* Priority glow
* Animated completion

---

# Background

Create an immersive animated background.

Include:

* Mesh gradients
* Aurora effect
* Floating blurry circles
* Radial gradients
* Animated color transitions
* Noise overlay

Everything should feel alive but subtle.

---

# Color Palette

Primary

* Indigo
* Violet

Accent

* Cyan
* Pink

Surface

rgba(255,255,255,0.12)

Dark Surface

rgba(255,255,255,0.06)

Borders

rgba(255,255,255,0.18)

Text

White

Secondary Text

White 70%

---

# CSS Effects

Use:

backdrop-filter

backdrop-blur

mask-image

mix-blend-mode

filter

drop-shadow

gradient-border

noise overlay

radial gradients

mesh gradients

glass reflections

soft glow

inner shadow

animated gradients

---

# UX

The interface should feel:

Elegant

Luxury

Relaxing

Smooth

Premium

Minimal

Clean

Modern

Highly interactive

Every interaction should delight the user.

---

# Task Features

Include:

* Add task
* Edit task
* Delete task
* Complete task
* Drag and drop
* Categories
* Priorities
* Due dates
* Search
* Filters
* Dark mode
* Calendar
* Kanban
* Analytics
* Pomodoro
* Local storage persistence

---

# Code Quality

* Modular architecture
* Reusable components
* TypeScript everywhere
* Clean folder structure
* Custom hooks
* Accessible
* Responsive
* Optimized rendering
* Lazy loading
* Mobile-first

---

# Tailwind Guidelines

Use Tailwind utility classes heavily.

Prefer:

* backdrop-blur-2xl
* bg-white/10
* bg-white/5
* border-white/20
* rounded-3xl
* shadow-2xl
* transition-all
* duration-500
* ease-out
* hover:scale-[1.02]
* hover:-translate-y-1
* hover:shadow-purple-500/20

---

# Final Goal

Generate a production-ready React application that looks like a premium SaaS product rather than a tutorial project. Every screen should feature layered glassmorphism, subtle grain textures, animated gradients, floating translucent panels, elegant typography, rich micro-interactions, and exceptional visual polish. The UI should feel futuristic, luxurious, and immersive while remaining highly usable, responsive, accessible, and performant.