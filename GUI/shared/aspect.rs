//! User-selected display proportions, independent of processing resolution.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AspectRatio { #[default] Auto, FourThree, SixteenNine, SixteenTen, FiveFour }
impl AspectRatio {
    pub const ALL: [Self; 5] = [Self::Auto, Self::FourThree, Self::SixteenNine, Self::SixteenTen, Self::FiveFour];
    pub fn parse(s: &str) -> Self { match s { "4:3" => Self::FourThree, "16:9" => Self::SixteenNine, "16:10" => Self::SixteenTen, "5:4" => Self::FiveFour, _ => Self::Auto } }
    pub fn name(self) -> &'static str { match self { Self::Auto => "auto", Self::FourThree => "4:3", Self::SixteenNine => "16:9", Self::SixteenTen => "16:10", Self::FiveFour => "5:4" } }
    /// SD sampling ratios describe a 704-pixel picture inside a 720-pixel raster.
    /// Recognize only the exact conventional metadata; never guess from SD size alone.
    pub fn aperture(self, size:(u32,u32), dar:(u32,u32))->(u32,u32) {
        if self==Self::Auto && size.0==720 && matches!(size.1,480|576) {
            let (n,d)=(dar.0 as u64,dar.1 as u64);
            if d>0 && (n*11==d*15 || n*11==d*20) {return (8,704);}
        }
        (0,size.0)
    }
    pub fn display(self,size:(u32,u32),dar:(u32,u32))->(u32,u32) {
        let (_,active)=self.aperture(size,dar);
        if active==704 && size.0==720 {
            return if dar.0 as u64*11==dar.1 as u64*15 {(4,3)}else{(16,9)};
        }
        self.dimensions(if dar.0>0&&dar.1>0 {dar}else{size})
    }
    pub fn dimensions(self, source: (u32,u32)) -> (u32,u32) {
        match self { Self::FourThree => (4,3), Self::SixteenNine => (16,9), Self::SixteenTen => (16,10), Self::FiveFour => (5,4), Self::Auto if source.0>0 && source.1>0 => source, _ => (16,9) }
    }
}
#[cfg(test)] mod tests {
    use super::*;
    #[test] fn saved_choices_and_old_settings() {
        for a in AspectRatio::ALL { assert_eq!(AspectRatio::parse(a.name()),a); }
        assert_eq!(AspectRatio::parse(""),AspectRatio::Auto);
        assert_eq!(AspectRatio::Auto.dimensions((720,576)),(720,576));
    }
    #[test] fn sd_auto_uses_active_aperture_without_guessing_other_formats() {
        let a=AspectRatio::Auto;
        assert_eq!(a.display((720,480),(15,11)),(4,3));
        assert_eq!(a.aperture((720,480),(15,11)),(8,704));
        assert_eq!(a.display((720,480),(20,11)),(16,9));
        assert_eq!(a.display((720,576),(15,11)),(4,3));
        assert_eq!(a.aperture((720,480),(4,3)),(0,720));
        assert_eq!(a.aperture((1920,1080),(15,11)),(0,1920));
        assert_eq!(a.aperture((704,480),(4,3)),(0,704));
        assert_eq!(a.aperture((720,480),(0,0)),(0,720));
        assert_eq!(AspectRatio::FourThree.aperture((720,480),(15,11)),(0,720));
        assert_eq!(AspectRatio::SixteenNine.display((720,480),(15,11)),(16,9));
        let (viewport,_)=crate::canvas::geometry(a.display((720,480),(15,11)),(1920,1080),(1920,1080));
        assert_eq!(viewport,[240,0,1440,1080]);
    }
    #[test] fn auto_follows_reported_ratios_and_falls_back_to_frame_dimensions() {
        let auto=AspectRatio::Auto;
        for (dar, expected) in [((16,10),[96,0,1728,1080]),((5,4),[285,0,1350,1080]),((16,9),[0,0,1920,1080])] {
            // Coded pixels need not have the same ratio as the broadcast display metadata.
            let ratio=auto.display((1920,1080),dar);
            assert_eq!(ratio,dar);
            assert_eq!(crate::canvas::geometry(ratio,(1920,1080),(1920,1080)).0,expected);
        }
        assert_eq!(auto.display((1920,1200),(0,0)),(1920,1200));
        assert_eq!(auto.display((1280,1024),(0,0)),(1280,1024));
        assert_eq!(AspectRatio::FiveFour.display((1920,1080),(16,10)),(5,4));
        assert_eq!(AspectRatio::SixteenTen.display((1920,1080),(5,4)),(16,10));
    }
    #[test] fn forced_ratios_fit_without_cropping() {
        for (a, expected) in [(AspectRatio::FourThree,[240,0,1440,1080]),(AspectRatio::SixteenNine,[0,0,1920,1080]),(AspectRatio::SixteenTen,[96,0,1728,1080]),(AspectRatio::FiveFour,[285,0,1350,1080])] {
            let (view,processing)=crate::canvas::geometry(a.dimensions((720,576)),(1920,1080),(2560,1440));
            assert_eq!(view,expected); assert_eq!(processing,(expected[2],expected[3]));
        }
    }
}
