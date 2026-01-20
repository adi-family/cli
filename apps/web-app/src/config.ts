// Application configuration

export const config = {
  // Minimum time (ms) to show loading states to prevent flickering
  minLoadingTime: 300,
};

// Helper to ensure minimum loading time
export async function withMinLoadingTime<T>(
  promise: Promise<T>,
  minTime: number = config.minLoadingTime
): Promise<T> {
  const start = Date.now();
  const result = await promise;
  const elapsed = Date.now() - start;
  
  if (elapsed < minTime) {
    await new Promise(resolve => setTimeout(resolve, minTime - elapsed));
  }
  
  return result;
}
