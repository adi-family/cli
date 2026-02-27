import html2canvas from 'html2canvas';

interface CaptureOptions {
  width: number;
  height: number;
  format?: 'image/jpeg' | 'image/png';
  quality?: number;
}

export async function captureFrame(
  element: HTMLElement,
  options: CaptureOptions,
): Promise<Blob> {
  const canvas = await html2canvas(element, {
    width: options.width,
    height: options.height,
    scale: 1,
    useCORS: true,
    allowTaint: false,
    backgroundColor: '#000000',
  });

  return new Promise<Blob>((resolve, reject) => {
    canvas.toBlob(
      blob => blob ? resolve(blob) : reject(new Error('Failed to capture frame')),
      options.format ?? 'image/jpeg',
      options.quality ?? 0.95,
    );
  });
}
