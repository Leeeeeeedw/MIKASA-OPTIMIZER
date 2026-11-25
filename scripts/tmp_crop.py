from PIL import Image
ref = Image.open(r'src\assets\design-ref.png')
print('ref', ref.size)
crops = {
    'tmp-crop-a.png': (0, 0, 235, 205),
    'tmp-crop-b.png': (6, 0, 230, 200),
    'tmp-crop-c.png': (0, 0, 235, 230),
}
for name, box in crops.items():
    im = ref.crop(box)
    im.save('artifacts\\' + name)
    print(name, im.size)
cur = Image.open(r'src\assets\young-mikasa-hero.png').convert('RGBA')
cur.crop((0, 0, 210, 146)).save(r'artifacts\tmp-crop-current.png')
print('current top', (210, 146))
