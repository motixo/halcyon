"""Fresh native editor outline calibration for focus only; no geometry claims."""
from calibrated_controls import Pixels

def locate(path):
    image=Pixels(path)
    class Grid:
        def __getitem__(self,xy):return image.rgb(*xy)
    px=Grid()
    def near(a,b,limit):return max(abs(x-y) for x,y in zip(a,b))<=limit
    def run(y,color,limit=8):
        left=right=640
        while left>10 and near(px[left-1,y],color,limit):left-=1
        while right<1270 and near(px[right+1,y],color,limit):right+=1
        return left,right
    for top in range(65,230):
        color=px[640,top]
        if min(color)<75 or max(color)<145:continue
        left,right=run(top,color)
        if not 600<=right-left<=1180:continue
        for bottom in range(top+280,711):
            if not near(px[640,bottom],color,18):continue
            bl,br=run(bottom,px[640,bottom],18)
            if min(right,br)-max(left,bl)<550:continue
            # Real tab/menu foreground within this outline, not wallpaper.
            bg=px[left+50,top+35]
            ink=sum(not near(px[x,y],bg,55) for y in range(top+20,min(top+100,bottom)) for x in range(left+20,min(left+300,right)))
            if ink<120:continue
            return {'x':left,'y':top,'width':right-left,'height':bottom-top,'source':'fresh outline calibration for focus only','header_foreground_pixels':ink}
    raise ValueError('Fresh calibrated editor outline not visible')
