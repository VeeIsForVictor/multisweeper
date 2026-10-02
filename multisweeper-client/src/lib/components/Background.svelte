<style>
    .scrolling-grid-bg {
        position: fixed;
        top: 0;
        left: 0;
        width: 100vw;
        height: 100vh;
        z-index: -1; /* Keeps it behind your content */
        background-color: white; /* Dark background color */
        
        /* Creates the grid lines using horizontal and vertical linear gradients */
        background-image: 
            linear-gradient(to right, rgba(180, 180, 180, 0.1) 1px, transparent 1px),
            linear-gradient(to bottom, rgba(180, 180, 180, 0.1) 1px, transparent 1px);
        
        /* Sets the size of each grid square (40px by 40px) */
        background-size: 40px 40px; 
        
        /* Attaches the infinite scrolling animation */
        animation: scrollGrid 2s linear infinite;
        }

        /* Animates the background position to match the background size for a seamless loop */
        @keyframes scrollGrid {
        from {
            background-position: 0 0;
        }
        to {
            background-position: 40px 40px; /* Must match the background-size width/height */
        }
    }
</style>

<div class="scrolling-grid-bg">

</div>