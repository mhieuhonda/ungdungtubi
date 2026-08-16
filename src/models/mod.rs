pub mod community;
pub mod friends;
pub mod kinh_sach;
pub mod khong_gian;
pub mod nha_nhac;
pub mod quy_tu_bi;
pub mod user;

#[allow(unused_imports)]
pub use user::{MemberRank, ProfileUpdate, User};

#[allow(unused_imports)]
pub use community::{
    Comment, CommentCreateForm, CommentWithAuthor, GlobalChatMessage, GlobalChatMessageWithAuthor,
    Group, GroupCategory, GroupCreateForm, GroupMember, GroupWithCategory,
    Topic, TopicCreateForm, TopicWithAuthor,
};

#[allow(unused_imports)]
pub use friends::{
    Conversation, ConversationWithParticipant, DirectMessage, DirectMessageWithAuthor,
    Friendship, FriendshipWithUser, Mail, MailWithUsers, Notification, NotificationWithActor,
};

#[allow(unused_imports)]
pub use kinh_sach::{
    Book, BookCategory, BookChapter, BookChapterSummary, BookReview, BookReviewForm,
    BookReviewWithAuthor, BookWithCategory,
};
