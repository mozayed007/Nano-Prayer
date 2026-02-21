import sys
from PIL import Image

def crop_transparent(image_path, output_path):
    img = Image.open(image_path)
    if img.mode != 'RGBA':
        img = img.convert('RGBA')

    # Get the bounding box of non-transparent (or non-whiteish) pixels
    # Since it's a transparency image, getbbox() gets the bounding box of non-zero alpha
    bbox = img.getbbox()
    if bbox:
        # Crop the image to the contents
        img_cropped = img.crop(bbox)
        
        # Calculate padding to make it a square
        width, height = img_cropped.size
        new_size = max(width, height)
        
        # Adding a modest 5% padding so it looks good on Windows/Mac
        padding = int(new_size * 0.05)
        final_size = new_size + (padding * 2)
        
        # Create a new transparent image
        new_img = Image.new('RGBA', (final_size, final_size), (0, 0, 0, 0))
        
        # Paste the cropped image into the center
        paste_x = (final_size - width) // 2
        paste_y = (final_size - height) // 2
        new_img.paste(img_cropped, (paste_x, paste_y))
        
        new_img.save(output_path)
        print(f"Successfully cropped and padded. Output saved to {output_path}")
    else:
        print("Could not find bounding box.")

if __name__ == "__main__":
    if len(sys.argv) != 3:
        print("Usage: python crop_icon.py <input> <output>")
        sys.exit(1)
    crop_transparent(sys.argv[1], sys.argv[2])
